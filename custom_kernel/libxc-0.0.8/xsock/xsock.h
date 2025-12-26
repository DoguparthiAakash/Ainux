#ifndef H_XSOCK
#define H_XSOCK    ("1.0.0")

#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

   /* Wait for a tcp connection on the specified port,
    * waiting indefinitely. The fd of the
    * connected socket is returned.
    */
   int xsock_listen (size_t port, size_t timeout);

   /* Make a connection to the specified server on the
    * specified port.
    * The fd of the connected descriptor is returned.
    */
   int xsock_connect (char *server, size_t port);

   /* Close the given file descriptor. fd must have
    * been previously opened using a "sock_" command.
    */
   int xsock_close (int fd);

   /* Write the given buffer to the given fd, return number
    * of bytes successfully returned. On error -1 is
    * returned.
    */
   size_t xsock_write (int fd, void *buf, size_t len);

   /* Read not more than the specified number of bytes from
    * the given fd into the specified buffer. On error
    * -1 is returned. No more than 'timeout' seconds
    * is spent waiting for a message.
    */
   size_t xsock_read (int fd, void *buf, size_t len, size_t timeout);

   int xsock_clear_errno (void);
   int xsock_errno (void);
   const char *xsock_strerror (int err);

   /* Run a few tests, return the result. True argument
    * means to run the test as a server, false argument
    * means run the test as a client.
    */
   int xsock_test (bool do_server_test);

#ifdef __cplusplus
};
#endif


#endif
