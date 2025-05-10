#include <unistd.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/epoll.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <signal.h>

#define PORT 51717
#define MAX_CLIENT 10

volatile sig_atomic_t status = 0;
struct epoll_event server_event, client_events[MAX_CLIENT];

typedef struct {
    struct sockaddr_in addr;
    int fd;
} client_data_t;

static void sig_hand(int sig) {
    status = sig;
} 

static int listen_to(int port) {
    int serverfd = 0; // https://www.linuxhowtos.org/data/6/fd.txt
    if ((serverfd = socket(AF_INET, SOCK_STREAM, PF_UNSPEC)) < 0) {
        fprintf(stderr, "Error opening socket: %d\n", serverfd);
        return -1;
    }

    // Set socket to be nonblocking
    char on = 1;
    if (ioctl(serverfd, FIONBIO, &on) < 0) {
        fprintf(stderr, "Error on setting socket to be nonblocking");
        close(serverfd);
        return -1;
    }

    struct sockaddr_in server_addr;
    memset(&server_addr, 0, sizeof(server_addr));

    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(PORT);
    server_addr.sin_addr.s_addr = INADDR_ANY;
    printf("Server address: %s:%d\n", inet_ntoa(server_addr.sin_addr), ntohs(server_addr.sin_port));

    if (bind(serverfd, (struct sockaddr*) &server_addr, sizeof(server_addr)) < 0) {
        fprintf(stderr, "Error binding socket\n");
        close(serverfd);
        return -1;
    }
    
    if (listen(serverfd, 32) < 0) { 
        fprintf(stderr, "Error on listening\n");
        close(serverfd);
        return -1;
    }
    return serverfd;
}

static int init_epoll(int fd) {
    // Create epoll instance
    int epoll_fd = epoll_create(1);
    if (epoll_fd < 0) {
        fprintf(stderr, "Error on creating epoll instance\n");
        return -1;
    }
    
    // Set the epoll structure
    server_event.events = EPOLLIN | EPOLLOUT | EPOLLET;
    server_event.data.fd = fd;
    if (epoll_ctl(epoll_fd, EPOLL_CTL_ADD, fd, &server_event) < 0) {
        fprintf(stderr, "Error on creating epoll instance\n");
        close(epoll_fd);
        return -1;
    }
    return epoll_fd;
}

int main(int argc, char const** argv) {
    int server_fd = 0; 
    if ((server_fd = listen_to(PORT)) < 0) {
        exit(-1);
    }

    int epoll_fd = 0;
    if ((epoll_fd = init_epoll(server_fd)) < 0) {
        close(server_fd);
        exit(-1);
    }

    signal(SIGINT, sig_hand);   

    char buffer[1024];
    
    while(!status) {
        int n_ev = 0;
        if ((n_ev = epoll_wait(epoll_fd, client_events, MAX_CLIENT, 60 * 1000)) < 0) break;
        for (int i = 0; i < n_ev; ++i) {
            if (client_events[i].data.fd == server_fd) {
                struct sockaddr_in client_addr;
                socklen_t client_len = sizeof(client_addr);
                int clientfd = accept(server_fd, (struct sockaddr*) &client_addr, &client_len);
                if (clientfd < 0) {
                    fprintf(stderr, "Error on accept: %d\n", clientfd);
                    continue;
                }
            
                char const* client_ip_addr = inet_ntoa(client_addr.sin_addr);
                printf("Client address: %s:%d\n", client_ip_addr, ntohs(client_addr.sin_port));

                struct epoll_event client_event;

                client_data_t* client_data = malloc(sizeof(client_data_t));
                client_data->addr = client_addr;
                client_data->fd = clientfd;
                client_event.events = EPOLLIN | EPOLLET | EPOLLRDHUP | EPOLLHUP;
                client_event.data.ptr = client_data;
                if (epoll_ctl(epoll_fd, EPOLL_CTL_ADD, clientfd, &client_event) < 0) {
                    fprintf(stderr, "Error on adding client\n");
                }
                continue;
            }

            if (client_events[i].events & EPOLLIN) {
                client_data_t* client_data = (client_data_t*)client_events[i].data.ptr;
                for(;;) {
                    memset(buffer, 0, sizeof(buffer));
                    int n = read(client_data->fd, buffer, sizeof(buffer));
                    if (n <= 0)
                        break;
                    else printf("Client %s:%d : %s\n", inet_ntoa(client_data->addr.sin_addr), ntohs(client_data->addr.sin_port), buffer);
                }
            }

            if (client_events[i].events & (EPOLLRDHUP | EPOLLHUP)) {
                client_data_t* client_data = (client_data_t*)client_events[i].data.ptr;
                printf("Connection closed from client %s:%d\n", inet_ntoa(client_data->addr.sin_addr), ntohs(client_data->addr.sin_port));
                epoll_ctl(epoll_fd, EPOLL_CTL_ADD, client_data->fd, NULL);
                close(client_data->fd);
                free(client_data);
            }
        }
    }

    close(epoll_fd);
    close(server_fd);
    return 0;
}