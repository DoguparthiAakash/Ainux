# 
# Auto generated file, edit to your taste
# 

BINPROG=xlex$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)

OBS=\
	main.o\
	xlex.o\
	..$(DS)xerror$(DS)xerror.o\
	..$(DS)xstring$(DS)xstring.o\
	..$(DS)xvector$(DS)xvector.o

HEADERS=\
	xlex.h\
	..$(DS)xdict$(DS)xdict.h\
	..$(DS)xstring$(DS)xstring.h\

all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS) $(LD_EXTRAFLAGS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

..$(DS)xvector$(DS)xvector.o:
	$(MAKE) -C ..$(DS)xvector

..$(DS)xerror$(DS)xerror.o:
	$(MAKE) -C ..$(DS)xerror

..$(DS)xstring$(DS)xstring.o:
	$(MAKE) -C ..$(DS)xstring

clean:
	$(RM) $(BINPROG) $(OBS)

