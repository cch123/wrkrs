# wrkrs

Same level performance as wrk:

```
~ ❯❯❯ wrk -c100 -t6 http://localhost:9090
Running 10s test @ http://localhost:9090
  6 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.10ms  150.82us   2.73ms   81.74%
    Req/Sec    14.59k   613.61    16.74k    74.59%
  879521 requests in 10.10s, 93.94MB read
Requests/sec:  87077.10
Transfer/sec:      9.30MB
```


