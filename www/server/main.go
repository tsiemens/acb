package main

import (
	"fmt"
	"net/http"
	"os"
	"path/filepath"
)

func main() {
	driverDir := os.Getenv("SERVER_DRIVER_DIR")
	assetsDir := filepath.Join(driverDir, "../html")

	fmt.Println("This server is for debuging/local use only!")
	fmt.Printf("Starting server for %s at localhost:9090. Use Ctrl-C to stop.", assetsDir)
	err := http.ListenAndServe(":9090", http.FileServer(http.Dir(assetsDir)))
	if err != nil {
		fmt.Println("Failed to start server", err)
		return
	}
}
