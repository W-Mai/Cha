package main

import (
	"fmt"
	"os"
)

type Server struct {
	Host string
	Port int
}

type Handler interface {
	Handle(req string) string
}

func (s *Server) Start() error {
	if s.Port <= 0 {
		return fmt.Errorf("invalid port")
	}
	fmt.Printf("Starting on %s:%d\n", s.Host, s.Port)
	return nil
}
