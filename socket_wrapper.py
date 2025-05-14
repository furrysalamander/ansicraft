#!/usr/bin/env python3

import socket
import subprocess
import os
import sys
import argparse
import pty
import select
import termios
import struct
import fcntl
import signal
import time
import array
import tty

def parse_args():
    parser = argparse.ArgumentParser(description='TCP socket wrapper for launch.sh')
    parser.add_argument('--host', default='0.0.0.0', help='Host address to bind to')
    parser.add_argument('--port', type=int, default=9867, help='Port to listen on')
    return parser.parse_args()

def set_terminal_size(fd, columns, lines):
    """Set the terminal size for the given file descriptor"""
    try:
        size = struct.pack("HHHH", lines, columns, 0, 0)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, size)
    except:
        pass

def handle_client(client_socket):
    """Handle a client connection by running launch.sh with I/O redirected to the socket"""
    print(f"Client connected from {client_socket.getpeername()}")
    
    # Send terminal initialization sequence
    client_socket.sendall(b"\x1b[?1049h")  # Enter alternate screen
    client_socket.sendall(b"\x1b[?25l")    # Hide cursor
    client_socket.sendall(b"\x1b[2J")      # Clear screen
    
    # Create a pseudo-terminal pair
    master_fd, slave_fd = pty.openpty()
    
    # Set default terminal size (will be resized by client if needed)
    columns, lines = 80, 43  # Larger default size for better viewing
    set_terminal_size(master_fd, columns, lines)
    
    # Launch the process with the slave side of the pty as stdin/stdout/stderr
    process = subprocess.Popen(
        ["./launch.sh"],
        stdin=slave_fd,
        stdout=slave_fd,
        stderr=slave_fd,
        close_fds=True,
        preexec_fn=os.setsid,  # Create new process group
        env=dict(os.environ, TERM="xterm-256color", COLORTERM="truecolor")  # Set terminal type with color support
    )
    
    # Close the slave side of the pty in the parent process
    os.close(slave_fd)
    
    # Set non-blocking mode on master pty and client socket
    fcntl.fcntl(master_fd, fcntl.F_SETFL, os.O_NONBLOCK)
    client_socket.setblocking(False)
    
    try:
        # Loop until the process ends or the client disconnects
        running = True
        while running:
            # Use select to wait for data on either the pty or socket
            readable, _, exceptional = select.select([master_fd, client_socket], [], [master_fd, client_socket], 1.0)
            
            # Read from the pty and send to the socket
            if master_fd in readable:
                try:
                    data = os.read(master_fd, 4096)  # Increased buffer size
                    if data:
                        client_socket.sendall(data)
                    else:
                        # End of file
                        running = False
                except OSError:
                    running = False
            
            # Read from the socket and write to the pty
            if client_socket in readable:
                try:
                    data = client_socket.recv(4096)  # Increased buffer size
                    if data:
                        # Check for terminal resize escape sequence
                        if b'\x1b[' in data and b't' in data:
                            try:
                                # Try to extract terminal size from sequence
                                seq_idx = data.find(b'\x1b[8;')
                                if seq_idx >= 0:
                                    end_idx = data.find(b't', seq_idx)
                                    if end_idx > seq_idx:
                                        parts = data[seq_idx+3:end_idx].decode().split(';')
                                        if len(parts) == 2:
                                            lines, columns = int(parts[0]), int(parts[1])
                                            set_terminal_size(master_fd, columns, lines)
                            except:
                                pass  # Ignore errors in terminal resize parsing
                        os.write(master_fd, data)
                    else:
                        # Client disconnected
                        running = False
                except ConnectionResetError:
                    running = False
            
            # Check for exceptions
            if exceptional:
                running = False
            
            # Check if process is still running
            if process.poll() is not None:
                running = False
    
    finally:
        # Clean up
        print(f"Client disconnected from {client_socket.getpeername()}")
        
        # Send terminal reset sequence
        try:
            client_socket.sendall(b"\x1b[?25h")  # Show cursor
            client_socket.sendall(b"\x1b[?1049l")  # Exit alternate screen
        except:
            pass
        
        # First try to send a Ctrl+C to the process (SIGINT)
        if process.poll() is None:
            try:
                print("Sending Ctrl+C (SIGINT) to process...")
                # Method 1: Send literal Ctrl+C character through the pty
                os.write(master_fd, b'\x03')  # Ctrl+C character
                
                # Method 2: Send SIGINT signal to process group
                os.killpg(os.getpgid(process.pid), signal.SIGINT)
                
                # Wait for the process to gracefully exit
                for _ in range(10):  # Wait up to 5 seconds
                    time.sleep(0.5)
                    if process.poll() is not None:
                        print("Process terminated successfully with Ctrl+C")
                        break
                
                # If Ctrl+C didn't work, try SIGTERM
                if process.poll() is None:
                    print("Ctrl+C didn't terminate process, trying SIGTERM...")
                    os.killpg(os.getpgid(process.pid), signal.SIGTERM)
                    time.sleep(2)
                    
                    # If still running, use SIGKILL as last resort
                    if process.poll() is None:
                        print("Process still running, using SIGKILL...")
                        os.killpg(os.getpgid(process.pid), signal.SIGKILL)
            except OSError as e:
                print(f"Error while terminating process: {e}")
        
        # Close the master pty and client socket
        os.close(master_fd)
        client_socket.close()

def main():
    args = parse_args()
    
    # Create a TCP socket
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    
    # Allow reuse of the address
    server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    
    # Bind to the specified host and port
    server_socket.bind((args.host, args.port))
    
    # Listen for incoming connections
    server_socket.listen(5)
    
    print(f"Server listening on {args.host}:{args.port}")
    
    try:
        while True:
            # Accept a connection
            client_socket, client_address = server_socket.accept()
            print(f"Accepted connection from {client_address}")
            
            # Handle the client in the current process
            handle_client(client_socket)
            
    except KeyboardInterrupt:
        print("Server shutting down")
    finally:
        server_socket.close()

if __name__ == "__main__":
    main()