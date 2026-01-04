#
# Small test program to test my c/p 
# shared object module.
#

#
# In a real project, I suggest you 
# compile using a different makefile
# (i.e. include the slib.* files into
# your build process, don't run the
# build in this directory using this
# makefile and then just copying the
# objects - a world of pain lies in that
# direction).
#
# Consider this makefile just as an example
# of the commands needed to run for the
# different platforms.

ifndef OSLIBTYPE
$(error  please set either OSLIBTYPE to "dll" or "so" to build)
endif

ifeq ($(OSLIBTYPE), so)
BINPROG=xshare$(BINEXT)
LIBLD=ld
LIBLDFLAGS= -Bdynamic -shared
LIBSUFFIX=so
DEFED=SO_LIB
endif

ifeq ($(OSLIBTYPE), dll)
BINPROG=xshare.exe
LIBLD=gcc 
LIBLDFLAGS= -shared 
LIBSUFFIX=dll
DEFED=DLL_LIB
endif

ifndef DEFED
$(error "Library type $(OSLIBTYPE) not supported")
endif

CC=gcc
CFLAGS= -c -ggdb -W -Wall -D$(DEFED) -I.. -fPIC -std=c99 $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS= -o$(BINPROG) $(LD_EXTRAFLAGS) 
ARCHIVE_CREATE= ar r


RM=rm -rf

OBS=\
   xshare.o \
	main.o \
   dynamic.o

HEADERS=\
   xshare.h \
	dynamic.h 

all:	$(OBS) libdynamic.$(LIBSUFFIX) 
	$(LD) $(LDFLAGS) main.o xshare.o $(LD_EXTRAFLAGS)

lib%.a:	%.o 
	$(ARCHIVE_CREATE) $@ $<

libdynamic.$(LIBSUFFIX): dynamic.o
	$(LIBLD) $(LIBLDFLAGS) dynamic.o -o libdynamic.$(LIBSUFFIX) 

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:
	$(RM) $(BINPROG) $(OBS) *.$(LIBSUFFIX) *.a *.o


