/**
 * \file 
 *
 * \brief Cross-platform BSD Sockets wrapper
 *
 * A collection of functions intended to provide easier TCP stream
 * usage within C programs. This module provides limited functionality;
 * callers can connect to a remote address:port, listen on a local
 * port and read/write to connected socket descriptors. Limited though
 * it is, this functionality is enough to perform communications over TCP
 * and/or provide services over TCP.
 *
 * For more fine-grained functionality, such as non-blocking IO, RAW 
 * datagrams, etc, use your platforms' native sockets
 * implementation directly.
 *
 * \sa xsock_listen(), xsock_connect(), xsock_close(), xsock_write(),
 *    xsock_read(), xsock_errno(), xsock_clear_errno(), xsock_strerror()
 *
 * xsock is part of the libxc (Extended C Library) and falls
 * under the relevant copyright license in libxc.
 *
 * \author Lelanthran Krishna Manickum
 *
 */
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <inttypes.h>

#include "xsock/xsock.h"



#include "xerror/xerror.h"

#ifdef WDEBUG
#define OUTPUT(...)        do {\
   XLOG (__VA_ARGS__); \
} while (0);
#else
#define OUTPUT(...)        ;
#endif

/* ***************************************************************** */
#if defined (PLATFORM_MSYS) || (PLATFORM_WINDOWS) || (P_windows_51)
#include <winsock2.h>
#include <windows.h>
#include <winsock.h>

#define SOCK_CLOEXEC       (0)
#define MSG_DONTWAIT       (0)
#define close(x)           closesocket (x)
#define accept4(x,y,z,A)   accept (x,y,(int *)z)
#define write(x,y,z)       send (x,y,z, 0)
#define read(x,y,z)        recv (x,(char *)y,z, 0)

typedef size_t socklen_t;

static WSADATA xp_wsaData;
static int xp_wsaInitialised = 0;
static void xp_winInit (void) {
   if (xp_wsaInitialised > 0) return;
   atexit ((void (*)(void))WSACleanup);
   int result = WSAStartup (MAKEWORD(2,2), &xp_wsaData);
   if (result!=0) {
      OUTPUT ("Critical: winsock could not be initiliased - %i\n", result);
   } else {
      OUTPUT ("winsock initialised\n");
   }

   xp_wsaInitialised = 1;
}

#define SAFETY_CHECK       do {\
   if (xp_wsaInitialised==0) xp_winInit (); \
} while (0)

#endif

/* ***************************************************************** */
#ifdef P_linux
#include <unistd.h>

#define _GNU_SOURCE
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/ip.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <sys/select.h>
/* GCC in strict mode leaves out some libraries it really shouldn't.
 * Rather than spend the next twenty minutes figuring out which set
 * of flags or pragmas will make GCC use the arpa/inet header, I'll
 * just put this here.
 */
int accept4(int sockfd, struct sockaddr *addr, socklen_t *addrlen, int flags);

#define SAFETY_CHECK    ;

#endif


#ifndef SAFETY_CHECK
#error SAFETY_CHECK not defined - platform variable undefined?
#endif
#include "xsock/xsock.h"

/**
 * \brief Listen for a connection
 *
 * Wait no more than \a timeout seconds for a connection on port
 * \a port. If a connection is made then the socket descriptor of the
 * incoming connection is returned, otherwise -1 is returned and the
 * error value is set appropriately (see \a xsock_geterrno()).
 *
 * If \a timeout expires before a connection is made then the return value
 * is zero.
 *
 * \sa xsock_connect(), xsock_close(), xsock_write(),
 *    xsock_read(), xsock_errno(), xsock_clear_errno(), xsock_strerror()
 *
 * @param[in] port The port to listen on
 * @param[in] timeout The timeout in seconds to wait for a connection
 *
 * @return On success, the socket descriptor of the incoming connection is
 *    returned and can be used with \a xsock_read(), \a xsock_write() and 
 *    \a xsock_close(). On timeout, zero is returned. On error -1 is
 *    returned and \a xsock_errno() will return the specific error number.
 *
 * \a Example:
 * \verbatim
 int main (void)
 {
    #define TEST_PORT      (8080)
    xsock_clear_errno ();
    // Wait for incoming connection 
    int fd = xsock_listen (TEST_PORT, 10);
    if (fd<=0) {
       fprintf (stderr, "No connection received on %u\n", TEST_PORT);
       fprintf (stderr, "err %i, %s\n", xsock_errno (),
                xsock_strerror(xsock_errno()));
       return EXIT_FAILURE;
    }
    // Read a string from the incoming connection
    char buffer[255]; buffer[255] = 0;
    size_t bytes_read = xsock_read (fd, buffer, sizeof buffer, 5);
    if (bytes_read==0) {
       printf ("Timeout\n");
    } else if (bytes_read<0) {
       printf ("Error reading: %s\n", xsock_strerror(xsock_errno()));
    } else {
      printf ("Read '%s'\n", buffer);  
    }
    xsock_close (fd);
 }
  \endverbatim
 */
int xsock_listen (size_t port, size_t timeout)
{
   /* ****************************************
    * 1. Call socket() to create a socket 
    * 2. Call bind() to bind to a local address
    * 3. Call listen() to wait for incoming connection
    * 4. Call accept() to get the clients connection.
    */
   SAFETY_CHECK;
   struct sockaddr_in addr;
   struct sockaddr_in ret;
   socklen_t retlen = sizeof ret;

   addr.sin_family = AF_INET;
   addr.sin_addr.s_addr = INADDR_ANY;
   addr.sin_port = htons (port);
   static int fd = -1;
   if (port==0) {
      close (fd);
   }
   if (fd<0) {
      fd = socket (AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
      if (fd<0) {
         OUTPUT (stderr, "socket() failed\n");
         return -1;
      }

      if (bind (fd, (struct sockaddr *)&addr, sizeof addr)!=0) {
         OUTPUT (stderr, "bind() failed\n");
         close (fd); fd = -1;
         return -1;
      }
   }
   if (listen (fd, 1)!=0) {
      OUTPUT (stderr, "listen() failed\n");
      close (fd); fd = -1;
      return -1;
   }
   errno = 0;
   {
      fd_set fds;
      struct timeval tv = { timeout , 0 };
      FD_ZERO (&fds);
      FD_SET (fd, &fds);
      int r = select (fd + 1, &fds, &fds, &fds, &tv);
      if (r==0) {
         close (fd); fd = -1;
         return 0;
      }
   }
   return accept4 (fd, (struct sockaddr *)&ret, &retlen, SOCK_CLOEXEC);
}

/**
 * \brief Connect to the specified address
 *
 * Connect to the specified address \a server on port \a port. The address
 * \a server can be either an IP address or a domain name. A single 
 * connection attempt is made to connect to \a server on \a port only.
 *
 * \sa xsock_listen(), xsock_close(), xsock_write(),
 *    xsock_read(), xsock_errno(), xsock_clear_errno(), xsock_strerror()
 *
 * @param[in] server The network address/domain to connect to
 * @param[in] port The port on which to connect
 *
 * @return On success a connected socket descriptor is returned which can
 *    be used with \a xsock_read(), \a xsock_write() and \a xsock_close(). The
 *    caller is responsible for closing the socket descriptor with 
 *    \a xsock_close().  On error (size_t)-1 is returned and
 *   \a  xsock_errno() will return the specific error number.
 *
 * \a Example:
 * \verbatim
 int main (void)
 {
    #define TEST_ADDRESS   ("10.207.204.12")
    #define TEST_PORT      (8080)
    #define TEST_MSG       ("Hello World")
    xsock_clear_errno ();
    // Connect and send a message
    int fd = xsock_connect (TEST_ADDRESS, TEST_PORT);
    if (fd<=0) {
       fprintf (stderr, "Connection refused on %s:$zu\n", 
                        TEST_ADDRESS, TEST_PORT);
       fprintf (stderr, "err %i, %s\n", xsock_errno (),
                        xsock_strerror(xsock_errno()));
       return EXIT_FAILURE;
    }
    // Write the string to the connected socket descriptor
    size_t bytes_written = xsock_write (fd, buffer, sizeof buffer);
    if (bytes_read==0) {
       printf ("No data written\n");
    } else if (bytes_read<0) {
       printf ("Error writing: %s\n", xsock_strerror(xsock_errno()));
    } else {
      printf ("Wrote '%zu'\n", bytes);  
    }
    xsock_close (fd);
 }
  \endverbatim
 */
int xsock_connect (char *server, size_t port)
{
   /* ****************************************
    * 0. Resolve the server name.
    * 1. Call socket() to create a new socket.
    * 2. Call connect() to connect to a remote server.
    */
   SAFETY_CHECK;
   // Resolving server name
   struct hostent *serv_addr = gethostbyname (server);
   struct sockaddr_in addr;
   addr.sin_family = AF_INET;
   addr.sin_port = htons (port);
   addr.sin_addr.s_addr = inet_addr (server);
   memcpy (&addr.sin_addr.s_addr, serv_addr->h_addr_list[0], 
         sizeof addr.sin_addr.s_addr);

   // Creating socket endpoint
   int fd = socket (AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
   if (fd<0) return -1;

   // Connecting endpoint to the server
   if (connect (fd, (struct sockaddr *)&addr, sizeof addr)!=0) {
      close (fd);
      return -1;
   }

   // Returning the connected fd to be used for reading/writing
   return fd;
}

/**
 * \brief Close a socket descriptor
 *
 * Closes the socket descriptor \a fd, which must have been returned from
 * a successful call to \a xsock_listen() or \a xsock_close(). On error -1
 * is returned and the specific error number can be retrieved with
 * \a xsock_errno().
 *
 * \sa xsock_listen(), xsock_connect(), xsock_close(), xsock_write(),
 *    xsock_read(), xsock_errno(), xsock_strerror()
 *
 * @return On success, zero is returned. On error, -1 is returned and the
 *    specific error number can be retrieved with \a xsock_errno().
 */
int xsock_close (int fd)
{
   /* ****************************************
    * 1. Call close() on the socket descriptor
    */
   SAFETY_CHECK;
   return close (fd);
}

/**
 * \brief Write data to a connected socket
 *
 * Write \a len bytes of data in buffer \a buf to socket descriptor \a fd.
 * The number of bytes actually written is returned. If an error occurs
 * (size_t)-1 is returned and the specific error number can be retrieved
 * with \a xsock_errno(). The write is performed in blocking mode.
 *
 * \sa xsock_listen(), xsock_close(), xsock_connect(),
 *    xsock_read(), xsock_errno(), xsock_clear_errno(), xsock_strerror()
 *
 * @param[in] fd The socket descriptor to write to
 * @param[in] buf The data to be written
 * @param[in] len The number of bytes to write
 *
 * @return On success the number of bytes actually written to the socket
 *    descriptor is returned, or zero if no data is written.
 *    On error (size_t)-1 is returned and \a xsock_errno() will return the
 *    specific error number.
 *
 * \a Example:
 * \verbatim
 int main (void)
 {
    #define TEST_ADDRESS   ("10.207.204.12")
    #define TEST_PORT      (8080)
    #define TEST_MSG       ("Hello World")
    xsock_clear_errno ();
    // Connect and send a message
    int fd = xsock_connect (TEST_ADDRESS, TEST_PORT);
    if (fd<=0) {
       fprintf (stderr, "Connection refused on %s:$zu\n", 
                        TEST_ADDRESS, TEST_PORT);
       fprintf (stderr, "err %i, %s\n", xsock_errno (),
                        xsock_strerror(xsock_errno()));
       return EXIT_FAILURE;
    }
    // Write the string to the connected socket descriptor
    size_t bytes_written = xsock_write (fd, buffer, sizeof buffer);
    if (bytes_read==0) {
       printf ("No data written\n");
    } else if (bytes_read<0) {
       printf ("Error writing: %s\n", xsock_strerror(xsock_errno()));
    } else {
      printf ("Wrote '%zu'\n", bytes);  
    }
    xsock_close (fd);
 }
  \endverbatim
 */
size_t xsock_write (int fd, void *buf, size_t len)
{
   SAFETY_CHECK;
   OUTPUT (stderr, "sending %i bytes\n", len);
   int64_t retval = write (fd, buf, len);
   if (retval<0) return (size_t)-1;
   return retval;
}

/**
 * \brief Read data from given connected and live socket
 *
 * Read not more than \a len bytes into buffer \a buf from socket descriptor
 * \a fd over the next \a timeout seconds. Should the timeout expire
 * before the buffer is filled then the number of bytes read in is returned.
 * If no bytes are read in after timeout expires then zero is returned. 
 * Should \a len bytes be read from the socket descriptor then the function
 * returns immediately without waiting for the timeout to expire.
 *
 * On failure -1 is returned and the specific error number can be retrieved
 * using \a xsock_errno().
 *
 * \sa xsock_listen(), xsock_connect(), xsock_close(), xsock_write(),
 *    xsock_errno(), xsock_clear_errno(), xsock_strerror()
 *
 * @param[in] fd The socket descriptor to read
 * @param[in] buf The buffer to fill with data read in from \a fd
 * @param[in] len The length of the buffer \a buf in bytes
 * @param[in] timeout The timeout in seconds to wait for data to become 
 *    available to read
 *
 * @return On success, the number of actual bytes read from the file
 *    descriptor \a fd is returned.  On error (size_t)-1 is returned and
 *    \a xsock_errno() will return the specific error number.
 *
 * \a Example:
 * \verbatim
 int main (void)
 {
    #define TEST_PORT      (8080)
    xsock_clear_errno ();
    // Wait for incoming connection 
    int fd = xsock_listen (TEST_PORT, 10);
    if (fd<=0) {
       fprintf (stderr, "No connection received on %u\n", TEST_PORT);
       fprintf (stderr, "err %i, %s\n", xsock_errno (),
                xsock_strerror(xsock_errno()));
       return EXIT_FAILURE;
    }
    // Read a string from the incoming connection
    char buffer[255]; buffer[255] = 0;
    size_t bytes_read = xsock_read (fd, buffer, sizeof buffer, 5);
    if (bytes_read==0) {
       printf ("Timeout\n");
    } else if (bytes_read<0) {
       printf ("Error reading: %s\n", xsock_strerror(xsock_errno()));
    } else {
      printf ("Read '%s'\n", buffer);  
    }
    xsock_close (fd);
 }
  \endverbatim
 */
size_t xsock_read (int fd, void *buf, size_t len, size_t timeout)
{
   size_t idx = 0;
   struct timeval tv = { timeout , 0 };
   unsigned char *buffer = buf;
   int countdown = 2;
   int error_code = 0;
   socklen_t error_code_len = sizeof error_code;

   getsockopt (fd,
               SOL_SOCKET, SO_ERROR, &error_code, &error_code_len);
   if (error_code!=0) return (size_t)-1;
   SAFETY_CHECK;
   OUTPUT (stderr, "Attempting to read %i bytes\n", len);
   do {
      fd_set fds;
      FD_ZERO (&fds);
      FD_SET (fd, &fds);
      int selresult = select (fd + 1, &fds, NULL, NULL, &tv);
      if (selresult>0) {
         xsock_clear_errno ();
         ssize_t r = recv (fd, &buffer[idx], len-idx, MSG_DONTWAIT);
         if (xsock_errno ()) return (size_t)-1;
         if (r==-1) return (size_t)-1;
         if (r == 0) {
            countdown--;
         } else {
            idx += (size_t)r;
            OUTPUT (stderr, "read %i bytes\n", idx);
         }
      } 
      if (selresult==0) {
         return idx;
      }
      if (selresult<0) {
         return (size_t)-1;
      }
   } while (idx<len && countdown);
   return idx;
}

/**
 * \brief Clear the last error value
 *
 * Clears the last error that occurred. This function is not reentrant.
 *
 * \sa xsock_listen(), xsock_connect(), xsock_close(), xsock_write(),
 *    xsock_read(), xsock_errno(), xsock_strerror()
 *
 * @return On success, zero is returned. On error, -1 is returned and the
 *    error is unspecified.
 */
int xsock_clear_errno (void)
{
   SAFETY_CHECK;
   errno = 0;
   return 0;
}

/**
 * \brief Retrieve the last error number
 *
 * Returns the number of the last error that occurred. This function is not
 * reentrant and will return the last error that occurred, even if the
 * occurrence was in a different thread.
 *
 * \sa xsock_listen(), xsock_connect(), xsock_close(), xsock_write(),
 *    xsock_read(), xsock_clear_errno(), xsock_strerror()
 *
 * @return The value of the last error that occurred is returned.
 */
int xsock_errno (void)
{
   SAFETY_CHECK;
   return errno;
}

/**
 * \brief Turn the error number given into a human-readable string
 *
 * Changes the error number \a err into a human readable string that can
 * be displayed to the user. This function is not reentrant and the caller
 * should make a copy of the return value if the returned string is to be
 * used long after the call to this function is made.
 *
 * \sa xsock_listen(), xsock_connect(), xsock_close(), xsock_write(),
 *    xsock_read(), xsock_clear_errno(), xsock_errno()
 *
 * @return A non-modifiable string describing the specified error \a err in
 *    a human-readable manner.
 */
const char *xsock_strerror (int err)
{
   SAFETY_CHECK;
   return strerror (err);
}

#define TEST_URL           ("127.0.0.1")
#define TEST_PORT          (4560)
#define TEST_TIMEOUT       (5)
#define TEST_CLIENT        "This is a test message from client"
#define TEST_SERVER        "This is a test message from server"

static char xp_client_msg[] = TEST_CLIENT;
static char xp_server_msg[] = TEST_SERVER;

static int xp_test_client (void)
{
   xsock_clear_errno ();
   /* Connect to a server */
   int fd = xsock_connect (TEST_URL, TEST_PORT);
   if (fd==-1) {
      fprintf (stderr, "Could not connect to %s:%u after %u seconds\n",
            TEST_URL, TEST_PORT, TEST_TIMEOUT);
      fprintf (stderr, "err %i: %s\n", xsock_errno(), xsock_strerror(errno));
      return -1;
   }
   fprintf (stderr, "Connected to %s:%u\n", TEST_URL, TEST_PORT);

   /* Write some data to the server */
   size_t written = xsock_write (fd, xp_client_msg, sizeof xp_client_msg);
   if (written!=sizeof xp_client_msg) {
      fprintf (stderr, "Could not write '%s' message of %zubytes\n",
            xp_client_msg, sizeof xp_client_msg);
      fprintf (stderr, "Wrote only %zubytes\n", written);
      fprintf (stderr, "err %i: %s\n", xsock_errno(), xsock_strerror(errno));
   } else {
      fprintf (stderr, "Wrote '%s' = %zubytes\n", xp_client_msg, sizeof xp_client_msg);
   }

   /* Read some data back from the server */
   memset (xp_server_msg, 0, sizeof xp_server_msg);
   int received = xsock_read (fd, xp_server_msg, sizeof xp_server_msg, TEST_TIMEOUT);
   if (received==-2) {
      fprintf (stderr, "Timed out while waiting for data\n");
      return -1;
   }

   if (received!=sizeof xp_server_msg) {
      fprintf (stderr, "Received only %i bytes - '%s'\n", received, xp_server_msg);
   } else {
      fprintf (stderr, "Received all %i bytes - '%s'\n", received, xp_server_msg);
   }

   /* Close the socket */
   if (xsock_close (fd)!=0) {
      fprintf (stderr, "Could not close the socket - err %i, %s\n",
            xsock_errno (), xsock_strerror (errno));
      return -1;
   } else {
      fprintf (stderr, "Closed the socket successfully\n");
      return 0;
   }
}

static int xp_test_server (void)
{
   xsock_clear_errno ();
   memset (xp_client_msg, 0, sizeof xp_client_msg);
   /* Wait for incoming connection */
   int fd = xsock_listen (TEST_PORT, 10);
   if (fd<=0) {
      fprintf (stderr, "No connection received on %u\n", TEST_PORT);
      fprintf (stderr, "err %i, %s\n", xsock_errno (), xsock_strerror(errno));
      return -1;
   }

   fprintf (stderr, "Connected to client, waiting for data\n");
   int len = xsock_read (fd, xp_client_msg, sizeof xp_client_msg, TEST_TIMEOUT);
   if (len==-2) {
      fprintf (stderr, "Timed out while waiting for data\n");
   }
   if (len<=0) {
      fprintf (stderr, "err %i, %s\n", xsock_errno(), xsock_strerror(errno));
      xsock_close (fd);
      return -1;
   } 
   fprintf (stderr, "Read %i bytes, '%s'\n", len, xp_client_msg);
   len = xsock_write (fd, xp_server_msg, sizeof xp_server_msg);
   if (len!=sizeof xp_server_msg) {
      fprintf (stderr, "Could not write full %zu bytes, wrote %i\n", 
            sizeof xp_server_msg, len);
      xsock_close (fd);
      return -1;
   } else {
      fprintf (stderr, "wrote %i bytes\n", len);
      xsock_close (fd);
      return 0;
   }
}

int xsock_test (bool do_server_test)
{
   fprintf (stderr, "socklib, v%s, testing %s\n", H_XSOCK,
         do_server_test ? "server" : "client");
   return do_server_test ? xp_test_server () : xp_test_client ();
}

