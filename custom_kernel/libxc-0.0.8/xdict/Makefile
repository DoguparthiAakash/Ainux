# 
# Auto generated file, edit to your taste
# 

BINPROG=xdict$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)



OBS=\
	main.o\
	xdict.o\
	..$(DS)xvector$(DS)xvector.o \
	..$(DS)xerror$(DS)xerror.o \
	..$(DS)xstring$(DS)xstring.o 

HEADERS=\
	xdict.h\

all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

..$(DS)xerror$(DS)xerror.o:
	$(MAKE) -C ..$(DS)xerror

..$(DS)xvector$(DS)xvector.o:
	$(MAKE) -C ..$(DS)xvector

..$(DS)xstring$(DS)xstring.o:
	$(MAKE) -C ..$(DS)xstring

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:
	$(RM) $(BINPROG) $(OBS)

