package main

import (
	"io"
	"log"
	_ "net/http/pprof"

	"net/http"
//"time"
//"math/rand"
)

func sayhello(wr http.ResponseWriter, r *http.Request) {
	// 标准库没有办法在写入后，再从responsewriter里读出来
	// 蛋疼
	//	wr.WriteHeader(404)
	//	wr.WriteHeader(1)
	wr.Header()["Content-Type"] = []string{"application/json"}
	//wr.Header().Set()
//time.Sleep(time.Millisecond*time.Duration(rand.Intn(1000)))
//time.Sleep(time.Millisecond*time.Duration((10)))
	io.WriteString(wr, "hello")

	//	panic(1)

	//	fmt.Println(wr.Header())
}

func main() {
	http.HandleFunc("/", sayhello)
	err := http.ListenAndServe(":9090", nil)
	if err != nil {
		log.Fatal("ListenAndServe:", err)
	}
}
