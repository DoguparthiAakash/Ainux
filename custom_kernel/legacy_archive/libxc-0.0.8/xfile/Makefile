# 
# Auto generated file, edit to your taste
# 

BINPROG=xfile$(BINEXT)
CC=gcc
CFLAGS=-c -W -Wall -I.. -ggdb -std=c99 -pedantic\
	-DOSLIBTYPE=$(OSLIBTYPE) -DPLATFORM=$(PLATFORM) -D$(PLATFORM) \
	-DDS=$(DS) \
	$(CC_EXTRAFLAGS)
LD=gcc
LDFLAGS=-o$(BINPROG)  $(LD_EXTRAFLAGS) 

ifeq ($(PLATFORM),Windows)
LDFLAGS+= -lregex
endif



OTHEROBS=\
	..$(DS)xstring$(DS)xstring.o\
	..$(DS)xerror$(DS)xerror.o\
	..$(DS)xvector$(DS)xvector.o\
   ..$(DS)xcrypto$(DS)xcrypto.o\
	..$(DS)xcrypto$(DS)sha2.o


OBS=\
	main.o\
   xfile.o\
   $(OTHEROBS)


HEADERS=\
	xfile.h\

all:	$(OBS) $(HEADERS)
	$(LD) $(LDFLAGS) $(OBS) $(LDFLAGS)

%.o:	%.c $(HEADERS)
	$(CC) $(CFLAGS) $<

clean:
	$(RM) $(BINPROG) $(OBS)

$(OTHEROBS):
	$(MAKE) -C .. xerror
	$(MAKE) -C .. xstring
	$(MAKE) -C .. xvector
	$(MAKE) -C .. xcrypto
      
