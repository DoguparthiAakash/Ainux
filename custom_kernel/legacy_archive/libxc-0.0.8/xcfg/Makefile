# 
# Auto generated file, edit to your taste
# 

BINPROG=xcfg$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)

OBS=\
	main.o\
	xcfg.o\
	..$(DS)xdict$(DS)xdict.o\
	..$(DS)xstring$(DS)xstring.o\
	..$(DS)xvector$(DS)xvector.o

HEADERS=\
	xcfg.h\
	..$(DS)xdict$(DS)xdict.h\
	..$(DS)xstring$(DS)xstring.h\

all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

..$(DS)xvector$(DS)xvector.o:
	$(MAKE) -C ..$(DS)xvector

..$(DS)xdict$(DS)xdict.o:
	$(MAKE) -C ..$(DS)xdict

..$(DS)xstring$(DS)xstring.o:
	$(MAKE) -C ..$(DS)xstring

clean:
	$(RM) $(BINPROG) $(OBS)

