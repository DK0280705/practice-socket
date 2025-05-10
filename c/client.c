#include <unistd.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <signal.h>
#include <string.h>
#include <stdio.h>

#define PORT 51717

volatile sig_atomic_t status = 0;
static void sig_hand(int sig) {
    status = sig;
} 

int main(int argc, char const** argv) {
    int sockfd = socket(AF_INET, SOCK_STREAM, PF_UNSPEC);
    if (sockfd < 0) {
        fprintf(stderr, "Error opening socket: %d\n", sockfd);
        return 1;
    }

    signal(SIGINT, sig_hand);

    struct sockaddr_in server_addr;
    memset(&server_addr, 0, sizeof(server_addr));

    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(PORT);
    server_addr.sin_addr.s_addr = INADDR_ANY;

    if (connect(sockfd, (struct sockaddr*) &server_addr, sizeof(server_addr)) < 0) {
        fprintf(stderr, "Error connecting to server\n");
        close(sockfd);
	return 0;
    }

    struct sockaddr_in client_addr;
    int clientlen = sizeof(client_addr);
    if (getsockname(sockfd, (struct sockaddr*) &client_addr, &clientlen) < 0) {
        fprintf(stderr, "Cannot get socket name\n");
    } else {
        char* client_ip_addr = inet_ntoa(client_addr.sin_addr);
        printf("Client address: %s:%d\n", client_ip_addr, ntohs(client_addr.sin_port));
    }
    
    while(!status) {
        char buffer[1024];
        printf("input : ");
        fgets(buffer, sizeof(buffer), stdin);
        int c = strlen(buffer) - 1;
        buffer[c] = '\0';
        int n = write(sockfd, buffer, c + 1);
        if (n < 0) perror("Error sending message\n");
    }

    printf("Connection ended\n");
    close(sockfd);
    return 0;
}
