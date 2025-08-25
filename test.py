import subprocess
import sys
import argparse
from PIL import Image
import io


def read_exact(stream, n):
    buf = bytearray()
    while len(buf) < n:
        chunk = stream.read(n - len(buf))
        if not chunk:
            # EOF
            return None
        buf.extend(chunk)
    return bytes(buf)


def run_decoder(input_file):
    cmd = [
        "/Users/xmoe/Downloads/omvdecoder/target/release/omvdecoder", 
        "--input",
        input_file,
        "--output",
        "dummy",
        "--format",
        "piped-png"
        ]

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=0
    )

    stdout = proc.stdout

    def read_line():
        line = stdout.readline()
        if not line:
            return None
        return line.decode("utf-8", errors="replace").rstrip("\n")

    header = read_line()
    if header is None:
        print("No header received, decoder failed?")
        return
    print(f"[HEADER] {header}")

    while True:
        line = read_line()
        if line is None:
            break
        if line.startswith("FRAME"):
            pts_line = read_line()
            bytes_line = read_line()
            if pts_line is None or bytes_line is None:
                break
            pts = int(pts_line.split(" ", 1)[1])
            n = int(bytes_line.split(" ", 1)[1])
            png_bytes = png_bytes = read_exact(stdout, n)
            if png_bytes is None:
                print("Unexpected EOF while reading PNG payload")
                break
            if len(png_bytes) < n:
                print("Unexpected EOF while reading PNG payload")
                break
            try:
                img = Image.open(io.BytesIO(png_bytes))
                print(f"[FRAME] PTS={pts}, PNG: {img.width}x{img.height}, mode={img.mode}, info={img.info}")
            except Exception as e:
                print(f"[FRAME] PTS={pts}, Failed to parse PNG: {e}")

    proc.wait()
    if proc.returncode != 0:
        stderr = proc.stderr.read().decode("utf-8", errors="replace")
        print(f"Decoder exited with code {proc.returncode}, stderr:\n{stderr}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Test omvdecoder piped PNG output")
    parser.add_argument("--input", help="Input media file to pass to omvdecoder")

    args = parser.parse_args()
    run_decoder(args.input)
