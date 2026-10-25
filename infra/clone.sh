#!/usr/bin/env sh

if [ "$#" -ne 1 ]; then
  echo "expected 1 argument, got $#"
	echo "usage: $0 <repo>"
	exit 2
fi

base=https://github.com/HaDeSMonsta

case "$1" in
	quadtree)
		repo=3sp-quadtree
		data="simra.tar.gz dwd.tar.gz"
		;;
	fastquadtree)
		repo=3sp-fastquadtree
		data="simra.tar.gz"
		;;
	sedona)
		repo=3sp-sedona
		data="simra.tar.gz"
		;;
	*)
		echo "unknown repo: $1"
		exit 2
		;;
esac

git clone "$base/$repo.git" || exit 1

for archive in $data; do
	if [ ! -f "$HOME/$archive" ]; then
		echo "missing $HOME/$archive"
		exit 1
	fi

	tar -xzf "$HOME/$archive" -C "$repo"
done
