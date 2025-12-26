# 
# Auto generated file, edit to your taste
# 

BINPROG=xsparse$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)



OBS=\
	main.o\
	xsparse.o\
	..$(DS)xvector$(DS)xvector.o \
	..$(DS)xstring$(DS)xstring.o \
	..$(DS)xdict$(DS)xdict.o 

HEADERS=\
	xsparse.h\

all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

..$(DS)xstring$(DS)xstring.o:
	$(MAKE) -C ..$(DS)xstring

..$(DS)xvector$(DS)xvector.o:
	$(MAKE) -C ..$(DS)xvector

..$(DS)xdict$(DS)xdict.o:
	$(MAKE) -C ..$(DS)xdict

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:
	$(RM) $(BINPROG) $(OBS)

