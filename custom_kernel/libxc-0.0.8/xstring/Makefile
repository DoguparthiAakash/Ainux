# 
# Auto generated file, edit to your taste
# 

BINPROG=xstring$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)



OBS=\
	main.o\
	xstring.o\
	..$(DS)xvector$(DS)xvector.o\
	..$(DS)xerror$(DS)xerror.o


HEADERS=\
	xstring.h\


all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

..$(DS)xerror$(DS)xerror.o:
	$(MAKE) -C ..$(DS)xerror

..$(DS)xvector$(DS)xvector.o:
	$(MAKE) -C ..$(DS)xvector

clean:
	$(RM) $(BINPROG) $(OBS)

