package main

import (
    "fmt"
    "strings"
)

type Server struct {
    Host string
    Port int
}

func NewServer(host string, port int) *Server {
    return &Server{Host: host, Port: port}
}

func (s *Server) Start() error {
    addr := fmt.Sprintf("%s:%d", s.Host, s.Port)
    fmt.Println("Starting server on", addr)
    return nil
}

func formatName(first, last string) string {
    return strings.TrimSpace(first + " " + last)
}
