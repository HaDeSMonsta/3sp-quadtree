#!/usr/bin/env bash

exec > >(tee -a /var/log/startup.log) 2>&1

status () {
	printf "$1\n" > /root/status.md
}

# I want the sourcing setup when I ssh into it
status "install rust"
curl https://sh.rustup.rs -sSf | sh -s -- -y
source /root/.cargo/env

status "update"
apt update

status "upgrade"
apt upgrade -y

status "install pkgs"
apt install -y vim git btop make gcc hyperfine

status "setup project"
git clone --depth 1 https://git.hadesmedia.de/hades/quadtree.git /root/quadtree
cd quadtree
make release

status "done"
