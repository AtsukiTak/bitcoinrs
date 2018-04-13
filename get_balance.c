/*
 * 1) HTTP POSTメソッドからデータを取得
 * 2) JSON Request Schemaに沿ってデータを整理、bitcoind(?)に渡す
 * 3) bitcoind(?)からのResponseをJSON Response Schemaにparse
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netdb.h>

int main (int argc, char *argv[])
{
    /* Determine the port number to receive the request from clients.
     * If argv[3] is a string, atoi() returns 0, otherwise it returns a certain number.
     */
    int portNum = atoi(argv[3])>0?atoi(argv[3]):80;
    /* Determine the hostname from which we recieve the data.
     * If argv[2] is not given, set it as "localhost." Otherwise, set as argv[2]
     */
    char *host = strlen(argv[2])>0?argv[2]:"localhost";

    // hostent structure is basically a linked-list of addresses
    struct hostent *server;
    struct sockaddr_in serv_addr;
    int sockfd, bytes, received, total, message_size = 0;
    char *message, request[2048];

    // If the number of parameter is not eonugh, exit the function
    if (argc < 5) {
        puts("Parameters: <method> <host> <port> <path> [<data> [<headers>]]");
        exit(1);
    }

    // When HTTP POST is used, calculates the size of message
    if (!strcmp(argv[1], "POST")) {
        message_size += strlen(argv[1]);                       /* method */
        message_size += strlen(argv[2]);                       /* host */
        message_size += strlen("HTTP/2\r\n");                  /* HTTP version */
        message_size += strlen(argv[3]);                       /* port */
        message_size += strlen(argv[4]);                       /* path */
        for (int i = 6; i < argc; i++) {
            message_size += strlen(argv[i]) + strlen("\r\n");  /* headers */
        }
        message_size += strlen("\r\n");                        /* blank line */
        if (argc > 5)
            message_size+=strlen(argv[5]);                     /* body */
    }
    else {
        puts("Invalid HTTP request.");
        exit(1);
    }

    // Allocate memory for the incoming message
    message = malloc(message_size);

    // Copy the input to message
    if (!strcmp(argv[1], "POST")) {
        sprintf(message, "POST %s HTTP/2\r\nHost: %s:%s\r\n",  /* method + HTTP version */
                strlen(argv[4])>0?argv[4]:"/",                 /* path */
                argv[2],                                       /* host */
                argv[3]);                                      /* port */
        for (int i = 6; i < argc; i++) {                       /* headers */
            strcat(message, argv[i]);
            strcat(message, "\r\n");
        }
        strcat(message, "\r\n");                               /* blank line */
        if (argc > 5)
            strcat(message, argv[5]);                          /* body */
    }

    // just for test
    printf("Request:\n%s\n", message);

    // Create socket and handle errors
    if ((sockfd = socket(AF_INET, SOCK_STREAM, 0)) < 0) {
        perror("Error: Cannot open socket");
        exit(1);
    }

    // Look up the IP address
    if ((server = gethostbyname(host)) == NULL) {
        perror("Error: No such host");
        exit(1);
    }

    // Fill in the structure (serv_addr)
    memset(&serv_addr, 0, sizeof(serv_addr));
    serv_addr.sin_family = AF_INET;
    serv_addr.sin_port = htons(portNum);
    memcpy(&serv_addr.sin_addr.s_addr, server->h_addr, server->h_length);

    // Connect the socket
    if (connect(sockfd, (struct sockaddr *)&serv_addr, sizeof(serv_addr)) < 0) {
        perror("Error: Cannot connect to the socket");
        exit(1);
    }

    // Receive the request
    memset(request, 0, sizeof(request));
    total = sizeof(request) -1;
    received = 0;
    do {
        // Read from the socket
        bytes = read(sockfd, request+received, total-received);
        if (bytes < 0) {
            perror("Error: Cannot read request from the socket");
            break;
        }
        if (bytes == 0)
            break;
        received += bytes;
    } while (received < total);

    if (received == total) {
        perror("Error: Cannot store the complete request from the socket");
        exit(1);
    }

    /* TODO: Send the given request to bitcoid(?) */

    /* TODO: Send the response from bitcoind(?) to client */

    // Close the socket
    close(sockfd);

    free(message);

    return 0;
}
