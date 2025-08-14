import socket
from interface import *

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect(("192.168.178.170", 1234))

    # Clear all filters
    s.send(make_cmd("clearfilt"))

    # Define a filter for all datagrams, but only allow them through every 5 seconds
    s.send(make_cmd("pfilt,5000,***_****_****"))
    
    # Send a can data frame on the bus
    frame = CanFrame(0x3ff, data=bytes.fromhex("1a2b3c"))
    s.send(frame_to_send(frame))

    # Show all received frames
    while True:
        data = s.recv(1024)
        for datagram in data.split(b'\n'):
            try:
                print(from_received_frame(datagram + b'\n'))
            except LookupError:
                pass # ignore empty datagram errors
            except Exception as e:
                print(f"Got Error '{e}'")
