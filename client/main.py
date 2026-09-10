import socket

SRV_ADDR = "127.0.0.1"
PORT = 6379

def client():
    # Create a TCP socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    # Connect to the server
    sock.connect((SRV_ADDR, PORT))

    # Main loop
    while True:
        send = str(input("Enter command: "))
        sock.sendall(send.encode())
        recv = sock.recv(1024)
        print(recv.decode())

if __name__ == "__main__":
    client()