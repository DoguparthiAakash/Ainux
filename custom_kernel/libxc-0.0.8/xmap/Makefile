# 
# Auto generated file, edit to your taste
# 

BINPROG=xmap$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic\
	-DOSLIBTYPE=$(OSLIBTYPE) -DPLATFORM=$(PLATFORM) -D$(PLATFORM) \
	 $(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG)  $(LD_EXTRAFLAGS)

OBS=\
	main.o\
	xmap.o\
   ..$(DS)xerror$(DS)xerror.o\
   ..$(DS)xstring$(DS)xstring.o\
	..$(DS)xvector$(DS)xvector.o

HEADERS=\
	xmap.h\
   ..$(DS)xerror$(DS)xerror.h\
   ..$(DS)xstring$(DS)xstring.h\
   ..$(DS)xvector$(DS)xvector.h\

all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:
	$(RM) $(BINPROG) $(OBS)

..$(DS)xerror$(DS)xerror.o:  ..$(DS)xerror$(DS)xerror.c ..$(DS)xerror$(DS)xerror.h
	$(MAKE) -C ..$(DS) xerror

..$(DS)xvector$(DS)xvector.o:  ..$(DS)xvector$(DS)xvector.c ..$(DS)xvector$(DS)xvector.h
	$(MAKE) -C ..$(DS) xvector

..$(DS)xstring$(DS)xstring.o:  ..$(DS)xstring$(DS)xstring.c ..$(DS)xstring$(DS)xstring.h
	$(MAKE) -C ..$(DS) xstring
