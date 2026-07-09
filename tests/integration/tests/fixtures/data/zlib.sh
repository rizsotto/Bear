gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -I../zlib-1.3.1/ -c -o example.o ../zlib-1.3.1/test/example.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o adler32.o ../zlib-1.3.1/adler32.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o crc32.o ../zlib-1.3.1/crc32.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o deflate.o ../zlib-1.3.1/deflate.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o infback.o ../zlib-1.3.1/infback.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o inffast.o ../zlib-1.3.1/inffast.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o inflate.o ../zlib-1.3.1/inflate.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o inftrees.o ../zlib-1.3.1/inftrees.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o trees.o ../zlib-1.3.1/trees.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o zutil.o ../zlib-1.3.1/zutil.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o compress.o ../zlib-1.3.1/compress.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o uncompr.o ../zlib-1.3.1/uncompr.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o gzclose.o ../zlib-1.3.1/gzclose.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o gzlib.o ../zlib-1.3.1/gzlib.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o gzread.o ../zlib-1.3.1/gzread.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -c -o gzwrite.o ../zlib-1.3.1/gzwrite.c
ar rc libz.a adler32.o crc32.o deflate.o infback.o inffast.o inflate.o inftrees.o trees.o zutil.o compress.o uncompr.o gzclose.o gzlib.o gzread.o gzwrite.o 
(ranlib libz.a || true) >/dev/null 2>&1
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN  -o example example.o -L. libz.a
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -I../zlib-1.3.1/ -c -o minigzip.o ../zlib-1.3.1/test/minigzip.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN  -o minigzip minigzip.o -L. libz.a
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/adler32.o ../zlib-1.3.1/adler32.c
mv objs/adler32.o adler32.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/crc32.o ../zlib-1.3.1/crc32.c
mv objs/crc32.o crc32.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/deflate.o ../zlib-1.3.1/deflate.c
mv objs/deflate.o deflate.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/infback.o ../zlib-1.3.1/infback.c
mv objs/infback.o infback.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/inffast.o ../zlib-1.3.1/inffast.c
mv objs/inffast.o inffast.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/inflate.o ../zlib-1.3.1/inflate.c
mv objs/inflate.o inflate.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/inftrees.o ../zlib-1.3.1/inftrees.c
mv objs/inftrees.o inftrees.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/trees.o ../zlib-1.3.1/trees.c
mv objs/trees.o trees.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/zutil.o ../zlib-1.3.1/zutil.c
mv objs/zutil.o zutil.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/compress.o ../zlib-1.3.1/compress.c
mv objs/compress.o compress.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/uncompr.o ../zlib-1.3.1/uncompr.c
mv objs/uncompr.o uncompr.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/gzclose.o ../zlib-1.3.1/gzclose.c
mv objs/gzclose.o gzclose.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/gzlib.o ../zlib-1.3.1/gzlib.c
mv objs/gzlib.o gzlib.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/gzread.o ../zlib-1.3.1/gzread.c
mv objs/gzread.o gzread.lo
mkdir objs 2>/dev/null || test -d objs
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -include zconf.h -DPIC -c -o objs/gzwrite.o ../zlib-1.3.1/gzwrite.c
mv objs/gzwrite.o gzwrite.lo
gcc -shared -Wl,-soname,libz.so.1,--version-script,../zlib-1.3.1/zlib.map -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -o libz.so.1.3.1 adler32.lo crc32.lo deflate.lo infback.lo inffast.lo inflate.lo inftrees.lo trees.lo zutil.lo compress.lo uncompr.lo gzclose.lo gzlib.lo gzread.lo gzwrite.lo  -lc 
rm -f libz.so libz.so.1
ln -s libz.so.1.3.1 libz.so
ln -s libz.so.1.3.1 libz.so.1
rmdir objs
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -o examplesh example.o  -L. libz.so.1.3.1
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -o minigzipsh minigzip.o  -L. libz.so.1.3.1
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -I../zlib-1.3.1/ -D_FILE_OFFSET_BITS=64 -c -o example64.o ../zlib-1.3.1/test/example.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN  -o example64 example64.o -L. libz.a
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -I. -I../zlib-1.3.1/ -D_FILE_OFFSET_BITS=64 -c -o minigzip64.o ../zlib-1.3.1/test/minigzip.c
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN  -o minigzip64 minigzip64.o -L. libz.a
