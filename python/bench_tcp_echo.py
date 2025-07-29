import socket
from datetime import datetime, timedelta


HOST = "192.168.178.170"  # The server's hostname or IP address
PORT = 1234  # The port used by the server

delta = 2
end_time = datetime.now() + timedelta(seconds=delta)
received = b""
expected = b""
data = b"1234567890"*100

print(f"len(data) {len(data)}")

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect((HOST, PORT))
    while datetime.now() < end_time:
        s.sendall(data)
        expected += data
        data_received = s.recv(1024) 
        if len(data_received) != len(data):
            data_received += s.recv(1024)
        received += data_received

if received != expected:
    print(f"Error, received Data is not equal sent data")
    exit(1)


print(f"Bytes received {len(received)} bytes -> {2*(len(received)/delta)//1024} kByte/s")
