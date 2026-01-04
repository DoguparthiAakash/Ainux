# 
#

CC=gcc
CFLAGS= -c -W -Wall -ansi -std=c99 -pedantic -I.. -ggdb $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS= -lm -lpthread $(LD_EXTRAFLAGS)


default:	all

veryclean:	clean
	$(RM) -rfv  $(LIBDIR) $(BINDIR)


BINPROG=xerror$(BINEXT)

OBS=main.o\
   xerror.o

HEADERS=xerror.h

LDFLAGS+= -o$(BINPROG)

all:	$(BINPROG)

$(BINPROG): $(OBS)
	$(LD) $(LDFLAGS) $(OBS) 

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:	
	$(RM) $(OBS) $(BINPROG)

