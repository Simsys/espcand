class CanFrame:
    def __init__(
        self, 
        id: int, 
        data: bytes = b"", 
        extended: bool = False,
        remote_frame: bool = False,
        dlc: int = 0
    ):
        self._id = id
        self._data = data
        self._extended = extended
        self._remote_frame = remote_frame
        self._dlc = dlc

    @property
    def id(self):
        return self._id
    
    @property
    def data(self):
        return self._data
    
    @property
    def extended(self):
        return self._extended
    
    @property
    def remote_frame(self):
        return self._remote_frame
    
    @property
    def dlc(self):
        if self._remote_frame:
            return self._dlc
        else:
            return len(self._data)

    def __str__(self):
        if self._remote_frame:
            return f"<Can Remote Frame id {hex(self._id)}, dlc {self._dlc}, extended {self._extended}>"
        else:
            return f"<Can Data Frame id {hex(self._id)}, data 0x{self._data.hex()}, extended {self._extended}>"


def from_received_frame(data: bytes) -> CanFrame:
    start = data.find(b'$')
    if start < 0:
        raise LookupError("Start $ not found")
    end = data.find(b'\n') 
    if end < 0:
        raise Exception("End \\n not found")
    parts = data[start:end].split(b',')
    if len(parts) != 4:
        raise Exception(f"Not a valid datagram {data[start:end]}")

    if parts[0] != b"$rf":
        raise Exception("Not a received frame")

    id = int(parts[1], 16)

    info = int(parts[2], 16)
    extended = (info & 0x80) == 0x80
    remote = (info & 0x40) == 0x40
    dlc = (info & 0x0f)

    data = bytes.fromhex(parts[3].decode("utf-8"))

    return CanFrame(id, data=data, dlc=dlc, extended=extended, remote_frame=remote)

def frame_to_send(frame: CanFrame) -> bytes:
    id = frame.id
    info = 0
    if frame.extended:
        info |= 0x80
    if frame.remote_frame:
        info |= 0x40
    info |= frame.dlc
    data = frame.data

    return f"$fts,{id:x},{info:x},{data.hex()}\n".encode("utf-8")

def make_cmd(cmd: str) -> bytes:
    return f"${cmd}\n".encode("utf-8")


# some tests
if __name__ == "__main__":
    frame = CanFrame(0x12a, data=bytes.fromhex("1a2b3c"))
    print(frame)
    print(frame_to_send(frame))
    frame = CanFrame(0x12a, data=bytes.fromhex("1a2b3c"), extended=True)
    print(frame)
    print(frame_to_send(frame))
    frame = CanFrame(0x12a, dlc=5, remote_frame=True)
    print(frame)
    print(frame_to_send(frame))
    frame = CanFrame(0x12a, dlc=5, remote_frame=True, extended=True)
    print(frame)
    print(frame_to_send(frame))

    print(from_received_frame(b"$rf,12a,3,1a2b3c\n"))
    print(from_received_frame(b"$rf,12a,83,1a2b3c\n"))
    print(from_received_frame(b"$rf,12a,43,\n"))
    print(from_received_frame(b"$rf,12a,c3,\n"))

    print(make_cmd("pfilt,5000,***_****_****"))
