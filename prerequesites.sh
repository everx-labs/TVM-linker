apk add perl
wget https://ftp.gnu.org/gnu/glibc/glibc-2.28.tar.gz
tar -xzf glibc-2.28.tar.gz
cd glibc-2.28
mkdir glibc-build
cd glibc-build
apk add --no-cache make gcc gawk bison linux-headers libc-dev
../configure --prefix=/usr\--disable-profile --enable-add-ons\--libexecdir=/usr/lib --with-headers=/usr/include\--without-cvs --enable-static-pie
cat >/etc/ld.so.conf << "EOF" # Begin/etc/ld.so.conf/usr/local/lib/opt/lib/usr/lib/usr/lib64/usr/libexeС# End/etc/ld.so.conf EOF
make
make install
