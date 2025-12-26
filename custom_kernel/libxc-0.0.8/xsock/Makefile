# 
# Auto generated file, edit to your taste
# 


ifeq ($(OSLIBTYPE), dll)
DEFED=P_windows_51
EXTRA_LDFLAGS=-lws2_32
endif

ifeq ($(OSLIBTYPE), so)
DEFED=P_linux
EXTRA_LDFLAGS=
endif

ifndef DEFED
$(error "Library type $(OSLIBTYPE) not supported: define DEFED")
endif


BINPROG=xsock$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic -D$(DEFED) $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(EXTRA_LDFLAGS) $(LD_EXTRAFLAGS)

OBS=\
	main.o\
	xsock.o\


HEADERS=\
	xsock.h\


all:	$(BINPROG)

$(BINPROG):	$(OBS) $(HEADERS)
	$(LD) $(OBS) $(LDFLAGS) 

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:
	$(RM) $(BINPROG) $(OBS)

