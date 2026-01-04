
#
# Toplevel build file for libxc
#

LIBDIR=./lib
INCDIR=./include
DOCDIR=./docs
DOCS=html latex xml man
PREREQS=$(LIBDIR) $(INCDIR) $(DOCDIR)

PLATFORM?=$(shell ./platform_name.sh)
OSTYPE_MSYS=$(findstring Msys,$(PLATFORM))
OSTYPE_LINUX=$(findstring Linux,$(PLATFORM))
OSTYPE_FREEBSD=$(findstring FreeBSD,$(PLATFORM))
OSTYPE_WINDOWS=$(findstring Windows,$(PLATFORM))
VERSION=$(shell cat ./VERSION)

OSLIBTYPE=so
LD_EXTRAFLAGS=
CC_EXTRAFLAGS=-fPIC
OSTYPE=PLATFORM_POSIX
DS=/

ONWINDOWS=no
ifeq ($(OSTYPE_MSYS),Msys)
	ONWINDOWS=yes
endif
ifeq ($(OSTYPE_WINDOWS),Windows)
	ONWINDOWS=yes
endif

ifeq ($(ONWINDOWS),yes)
   OSLIBTYPE=dll
   LD_EXTRAFLAGS=-lws2_32
   CC_EXTRAFLAGS=-fPIC -D__USE_MINGW_ANSI_STDIO=1
   OSTYPE=PLATFORM_WINDOWS
endif
ifeq ($(OSTYPE_LINUX),Linux)
	LD_EXTRAFLAGS= -ldl
   OSTYPE=PLATFORM_POSIX
endif
ifeq ($(OSTYPE_FREEBSD),FreeBSD)
	LD_EXTRAFLAGS= -lc
   OSTYPE=PLATFORM_POSIX
endif

LIBNAME=xc
LIBEXT=$(OSLIBTYPE)
LIBFILE=$(LIBDIR)/lib$(LIBNAME)-$(VERSION).$(LIBEXT)

LD=gcc
LDFLAGS= -shared -o $(LIBFILE)


CURDIR=$(shell pwd)

SUBPROJS=\
	xcfg\
   xcrypto\
   xcob\
	xdict\
	xerror\
	xlex\
	xfile\
	xmalloc\
	xmap\
	xsarray\
	xshare\
	xsock\
	xsparse\
	xstring\
	xtree\
	xvector

SUBOBS=$(foreach DIR,$(SUBPROJS),$(DIR).o)

.PHONY:	show $(SUBPROJS) $(PREREQS) $(LIBFILE)

all: $(PREREQS) $(SUBPROJS) $(LIBFILE)

show:
	@echo Using the following variables
	@echo PLATFORM=$(PLATFORM)
	@echo VERSION=$(VERSION)
	@echo OSTYPE=$(OSTYPE)
	@echo OSLIBTYPE=$(OSLIBTYPE)
	@echo CC_EXTRAFLAGS=$(CC_EXTRAFLAGS)
	@echo LD_EXTRAFLAGS=$(LD_EXTRAFLAGS)

$(LIBFILE):	$(SUBPROJS)
	rm -rf $(LIBDIR)/*.o
	if [ ! -d "$(LIBDIR)" ] ; then mkdir -p $(LIBDIR) ; fi
	for X in $(SUBPROJS); \
		do mkdir -p $(INCDIR)/$$X; \
		cp -v $$X/$$X.o $(LIBDIR); \
		cp -v $$X/$$X.h $(INCDIR)/$$X/ ;\
	done; \
	cp xcrypto/sha2.o $(LIBDIR) ;\
	$(LD) $(LDFLAGS) $(LIBDIR)/* $(LD_EXTRAFLAGS) &&\
	rm -rf $(LIBDIR)/*.o &&\
	cp $(LIBFILE) $(LIBDIR)/lib$(LIBNAME).$(LIBEXT)

check_docprogs:
	which doxygen || exit -1
	which doxy2man || echo "Manual pages won't be produced, doxy2man not found"

documentation: $(PREREQS) check_docprogs
	[ ! -z "`which doxygen`" ] || echo doxygen not installed exit -1
	[ ! -z "`which doxy2man`" ] || echo doxy2man not installed exit -1
	doxygen && \
	   $(MAKE) -C latex && \
	   cd xml && \
	   for X in *[1-9]c.xml libxc*.xml; do doxy2man --novalidate $$X; done && \
		mv -v out ../man
	mv html $(DOCDIR)/html
	mv latex $(DOCDIR)/latex
	mv man $(DOCDIR)/man
	mv xml $(DOCDIR)/xml
	mkdir $(DOCDIR)/pdf
	cp $(DOCDIR)/latex/refman.pdf $(DOCDIR)/pdf/libxc.pdf
	tar -zcvf libxc-`cat VERSION`-doc.tar.gz docs


$(SUBPROJS):	$(PREREQS)
	export OSLIBTYPE="$(OSLIBTYPE)" PLATFORM="$(OSTYPE)" \
          LD_EXTRAFLAGS="$(LD_EXTRAFLAGS)" \
          CC_EXTRAFLAGS="$(CC_EXTRAFLAGS)" \
          DS=$(DS) \
          && $(MAKE) -C $@ 

$(PREREQS):
	mkdir -p $@

clean:
	rm -rfv $(PREREQS) $(DOCS)
	for X in $(SUBPROJS); do \
		export OSLIBTYPE=$(OSLIBTYPE) && $(MAKE) DS=$(DS) -C $$X clean; done

package:	VERSION
	./package_source.sh `cat VERSION`
