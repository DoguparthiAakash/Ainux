# 
#

CC=gcc
CFLAGS= -c -W -Wall -ansi -std=c99 -pedantic -I.. -ggdb $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS= -lm -lpthread $(LD_EXTRAFLAGS)


default:	all

veryclean:	clean
	$(RM) -rfv  $(LIBDIR) $(BINDIR)


BINPROG=xvector$(BINEXT)

OBS=main.o\
   xvector.o

HEADERS=xvector.h

LDFLAGS+= -o$(BINPROG)

all:	$(BINPROG)

$(BINPROG): $(OBS)
	$(LD) $(LDFLAGS) $(OBS) 

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:	
	$(RM) $(OBS) $(BINPROG)

