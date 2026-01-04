# 
# Auto generated file, edit to your taste
# 

BINPROG=xcob$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)



OBS=\
	main.o\
	xcob.o\
	xcobmgmt.o\
	..$(DS)xstring$(DS)xstring.o\
	..$(DS)xerror$(DS)xerror.o\
	..$(DS)xmap$(DS)xmap.o\
	..$(DS)xvector$(DS)xvector.o\
	..$(DS)xdict$(DS)xdict.o\


HEADERS=\
	xcob.h\
	xcobmgmt.h\
	..$(DS)xstring$(DS)xstring.h\
	..$(DS)xerror$(DS)xerror.h\
	..$(DS)xmap$(DS)xmap.h\
	..$(DS)xvector$(DS)xvector.h\
	..$(DS)xdict$(DS)xdict.h\


all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

..$(DS)xerror$(DS)xerror.o:
	$(MAKE) -C ..$(DS)xerror

..$(DS)xstring$(DS)xstring.o:
	$(MAKE) -C ..$(DS)xstring

..$(DS)xmap$(DS)xmap.o:
	$(MAKE) -C ..$(DS)xmap

..$(DS)xvector$(DS)xvector.o:
	$(MAKE) -C ..$(DS)xvector

..$(DS)xdict$(DS)xdict.o:
	$(MAKE) -C ..$(DS)xdict

clean:
	$(RM) $(BINPROG) $(OBS)

