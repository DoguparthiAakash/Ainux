# 
# Auto generated file, edit to your taste
# 

BINPROG=xmalloc$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)



OBS=\
	main.o\
	xmalloc.o\


HEADERS=\
	xmalloc.h\



all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:
	$(RM) $(BINPROG) $(OBS)

