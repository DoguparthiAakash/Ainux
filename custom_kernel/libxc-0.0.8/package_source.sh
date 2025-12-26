#!/bin/bash

function die () {
   echo $@
   exit
}

[ -z "$1" ] && die no version

make clean || die build process failed to clean

PNAME="libxc-$1"

mkdir -p $PNAME || die unable to create $PNAME

cp -Rv * $PNAME
for X in `find $PNAME | grep \.svn`; do 
   rm -rfv $X
done

rm -rfv $PNAME/$PNAME

tar -zcvf $PNAME-src.tar.gz $PNAME

