import minecraft_launcher_lib
import subprocess
import sys
import os
import argparse
import tempfile

def parse_arguments():
    parser = argparse.ArgumentParser(description='Minecraft Launcher Script')
    parser.add_argument('--download-only', action='store_true', 
                        help='Download Minecraft files only, then exit')
    return parser.parse_args()

# Minecraft version to use
minecraft_version = "1.21.5"
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
    "username": "docker",
    "uuid": "00000000-0000-0000-0000-000000000000",
    "token": ""
}

minecraft_command = minecraft_launcher_lib.command.get_minecraft_command(
    minecraft_version,
    minecraft_directory,
    options
)

# Create temporary files to capture stdout and stderr
with tempfile.TemporaryFile(mode="w+") as stdout_file, tempfile.TemporaryFile(mode="w+") as stderr_file:
    # Launch Minecraft with stdout and stderr redirected to temp files
    process = subprocess.Popen(minecraft_command, stdout=stdout_file, stderr=stderr_file)
    
    # Wait for the process to finish
    return_code = process.wait()
    
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
