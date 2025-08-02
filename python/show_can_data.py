import socket
from datetime import datetime, timedelta


HOST = "192.168.178.170"  # The server's hostname or IP address
PORT = 1234  # The port used by the server

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect((HOST, PORT))
    while True:
        print(s.recv(1024).decode("utf-8")) 
