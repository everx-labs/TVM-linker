apk add --no-cache make gcc linux-headers bsd-compat-headers gawk bison binutils coreutils diffutils gettext bash grep sed texinfo perl
wget https://ftp.gnu.org/gnu/glibc/glibc-2.28.tar.gz
tar -xzf glibc-2.28.tar.gz
cd glibc-2.28
mkdir glibc-build
cd glibc-build
../configure --prefix=/usr\--disable-profile --enable-add-ons\--libexecdir=/usr/bin --with-headers=/usr/include\--enable-static-pie
make
make install