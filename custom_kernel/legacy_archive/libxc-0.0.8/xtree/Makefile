# 
#

CC=gcc
CFLAGS=-fPIC -c -W -Wall -ansi -std=c99 -pedantic -I.. -ggdb $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=  $(LD_EXTRAFLAGS)


default:	all

BINPROG=xtree$(BINEXT)

OBS=main.o\
   xtree.o\
   ..$(DS)xvector$(DS)xvector.o

HEADERS=xtree.h

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

