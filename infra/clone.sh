#!/usr/bin/env sh

if [ "$#" -ne 1 ]; then
  echo "expected 1 argument, got $#"
	echo "usage: $0 <repo>"
	exit 2
fi

base=https://github.com/HaDeSMonsta

case "$1" in
	quadtree) repo=3sp-quadtree ;;
	fastquadtree) repo=3sp-fastquadtree ;;
	sedona) repo=3sp-sedona ;;
	*)
		echo "unknown repo: $1"
		exit 2
		;;
esac

git clone "$base/$repo.git"
