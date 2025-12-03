import argparse
import os
import socket

import cbor2

def default_unix_path() -> str:
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR")
    if runtime_dir:
        return os.path.join(runtime_dir, "pose-de-game.sock")
    return "/tmp/pose-de-game.sock"


def resolve_transport(value: str) -> str:
    if value == "auto":
        return "unix" if os.name == "posix" else "udp"
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description="Send sample pose packet")
    parser.add_argument(
        "--transport", choices=["auto", "unix", "udp"], default="auto"
    )
    parser.add_argument("--unix-path", default=default_unix_path())
    parser.add_argument("--udp-addr", default="127.0.0.1:45233")
    args = parser.parse_args()

    people_data = [
        {
            "keypoints": [
                [0.6020771861076355, 0.2547045350074768],
                [0.6209439039230347, 0.23340444266796112],
                [0.5800461769104004, 0.2330082505941391],
                [0.6467500329017639, 0.27474913001060486],
                None,
                [0.6963533759117126, 0.45109689235687256],
                [0.5077019929885864, 0.44847649335861206],
                [0.730774998664856, 0.6360486149787903],
                [0.4677988588809967, 0.6313403844833374],
                [0.7760822176933289, 0.5071136951446533],
                [0.46516045928001404, 0.5084625482559204],
                [0.6645041108131409, 0.8852798342704773],
                [0.5411442518234253, 0.8873101472854614],
                None,
                None,
                None,
                None,
            ],
            "right_hand_closed": None,
            "left_hand_closed": None,
            "person_png": None,
        }
    ]

    cbor_data = cbor2.dumps(people_data)
    transport = resolve_transport(args.transport)

    if transport == "unix":
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
            sock.connect(args.unix_path)
            length = len(cbor_data).to_bytes(4, "big")
            sock.sendall(length + cbor_data)
    else:
        host, port_str = args.udp_addr.rsplit(":", 1)
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
            sock.sendto(cbor_data, (host, int(port_str)))


if __name__ == "__main__":
    main()
