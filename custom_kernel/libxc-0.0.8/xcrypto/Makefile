# 
# Auto generated file, edit to your taste
# 

BINPROG=xcrypto$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic -DBYTE_ORDER=LITTLE_ENDIAN\
	-DOSLIBTYPE=$(OSLIBTYPE) -DPLATFORM=$(PLATFORM) -D$(PLATFORM) \
	$(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG) $(LD_EXTRAFLAGS)



OBS=\
	main.o\
	xcrypto.o\
	sha2.o\
	..$(DS)xerror$(DS)xerror.o



HEADERS=\
	xcrypto.h\
	sha2.h



all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

..$(DS)xerror$(DS)xerror.o:
	$(MAKE) -C ..$(DS)xerror

clean:
	$(RM) $(BINPROG) $(OBS)

