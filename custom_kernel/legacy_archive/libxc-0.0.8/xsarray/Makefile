# 
#

CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic\
	-DOSLIBTYPE=$(OSLIBTYPE) -DPLATFORM=$(PLATFORM) -D$(PLATFORM) \
	 $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=  $(LD_EXTRAFLAGS)


BINPROG=xsarray$(BINEXT)

OBS=main.o\
   xsarray.o\
   ..$(DS)xvector$(DS)xvector.o

HEADERS=xsarray.h

LDFLAGS+= -o$(BINPROG)

all:	$(BINPROG)

$(BINPROG): $(OBS)
	$(LD) $(LDFLAGS) $(OBS) 

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

..$(DS)xvector$(DS)xvector.o:
	$(MAKE) -C ..$(DS)xvector

clean:	
	$(RM) $(OBS) $(BINPROG)

