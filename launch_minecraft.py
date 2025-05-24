import minecraft_launcher_lib
import subprocess
import sys
import os
import argparse
import tempfile
import signal
import atexit

# Global variable to track the subprocess
minecraft_process = None

def signal_handler(sig, frame):
    """Handle signals by terminating the Minecraft subprocess"""
    if minecraft_process:
        print(f"\nReceived signal {sig}, terminating Minecraft...")
        minecraft_process.terminate()
        try:
            # Wait up to 3 seconds for process to terminate
            minecraft_process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            # If it doesn't terminate gracefully, force kill it
            print("Minecraft not responding to terminate signal, force killing...")
            minecraft_process.kill()
    sys.exit(0)

def cleanup_at_exit():
    """Ensure Minecraft is terminated when script exits"""
    if minecraft_process and minecraft_process.poll() is None:
        minecraft_process.terminate()
        try:
            minecraft_process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            minecraft_process.kill()

def parse_arguments():
    parser = argparse.ArgumentParser(description='Minecraft Launcher Script')
    parser.add_argument('--download-only', action='store_true', 
                        help='Download Minecraft files only, then exit')
    parser.add_argument('--server', '-s', 
                        help='Server address to connect to on launch (e.g., example.com:25565)')
    parser.add_argument('--username', '-u', default="docker",
                        help='Username to use when launching Minecraft (default: docker)')
    return parser.parse_args()

# Minecraft version to use
minecraft_version = "1.21.4"
# Directory for minecraft
minecraft_directory = "/root/.minecraft"

# Parse command line arguments
args = parse_arguments()

# Ensure game directory exists
os.makedirs(minecraft_directory, exist_ok=True)

# Configure game options (fullscreen and raw mouse input) only if options.txt doesn't exist
options_dir = os.path.join(minecraft_directory, "options.txt")
if not os.path.exists(options_dir):
    print("Creating options.txt file...")
    with open(options_dir, "w") as f:
        f.write("rawMouseInput:false\n")
        f.write("fullscreen:true\n")
        f.write("autoJump:true\n")
        f.write("graphicsMode:0\n")
        f.write("guiScale:0\n")
        f.write("maxFps:30\n")

# print("Downloading Minecraft...")
# Download/install the client
minecraft_launcher_lib.install.install_minecraft_version(minecraft_version, minecraft_directory)

# If download-only mode is specified, exit now
if args.download_only:
    print("Minecraft files downloaded successfully. Exiting.")
    sys.exit(0)

# print("Starting Minecraft...")
# Get the Minecraft command to launch the client
options = {
    "username": args.username,
    "uuid": "00000000-0000-0000-0000-000000000000",
    "token": "",
}

minecraft_command = minecraft_launcher_lib.command.get_minecraft_command(
    minecraft_version,
    minecraft_directory,
    options
)

# If server specified, add server connection parameters
if args.server:
    print(minecraft_command)
    minecraft_command.append("--quickPlayMultiplayer")
    minecraft_command.append(args.server)

print(minecraft_command)

# Register signal handlers for SIGINT and SIGTERM
signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)
# Register exit handler
atexit.register(cleanup_at_exit)

# Create temporary files to capture stdout and stderr
with tempfile.TemporaryFile(mode="w+") as stdout_file, tempfile.TemporaryFile(mode="w+") as stderr_file:
    # Launch Minecraft with stdout and stderr redirected to temp files
    minecraft_process = subprocess.Popen(minecraft_command, stdout=stdout_file, stderr=stderr_file)
    
    # Wait for the process to finish
    return_code = minecraft_process.wait()
    
    # If the process crashed (non-zero exit code), print stdout and stderr
    if return_code != 0:
        print(f"Minecraft crashed with exit code {return_code}. Output:")
        
        # Print stdout content
        stdout_file.seek(0)
        stdout_content = stdout_file.read()
        if stdout_content:
            print("===== STDOUT =====")
            print(stdout_content)
        
        # Print stderr content
        stderr_file.seek(0)
        stderr_content = stderr_file.read()
        if stderr_content:
            print("===== STDERR =====")
            print(stderr_content)
