import socket
from interface import *

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect(("192.168.178.170", 1234))

    # Clear all filters
    s.send(b"$clearfilt\n")

    # Define a filter for all datagrams, but only allow them through every 5 seconds
    s.send(b"$pfilt,5000,***_****_****\n")
    
    # Send a can data frame on the bus
    s.send(b"$fts,12a,3,1a2b3c\n")

    # Show all received frames
    while True:
        data = s.recv(1024)
        print(data.decode("utf-8"), end="")
