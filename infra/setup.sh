#!/usr/bin/env bash

exec > >(tee -a /var/log/startup.log) 2>&1

status () {
	printf "$1\n" > /root/status.md
}

status "install rust"
export RUSTUP_HOME=/usr/local/rustup
export CARGO_HOME=/usr/local/cargo
curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --no-modify-path
export PATH="/usr/local/cargo/bin:$PATH"
chmod -R a+rX /usr/local/rustup /usr/local/cargo
printf 'export RUSTUP_HOME=/usr/local/rustup\nexport PATH=/usr/local/cargo/bin:$PATH\n' > /etc/profile.d/rust.sh

status "update"
apt update

status "upgrade"
apt upgrade -y

status "install pkgs"
apt install -y vim git btop make just gcc hyperfine python3 python3-venv

status "done"
