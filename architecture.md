```mermaid
flowchart LR
    subgraph FE [Frontend]
        A[room a]
        B[room b]
        N[room n]
        A <--> B
    end

    subgraph BE [Backend]
        subgraph API
            GET[GET /stream/{room_id}]
            POST[POST /stream/{room_id}]
        end
        subgraph MQ[Message Queue]
            MQbuf[Buffering]
            MQrate[Rate control]
            MQpersist[Persistence]
        end
        subgraph CP[Connection Pool]
            CPact[Active sockets]
            CPhealth[Health monitoring]
            CPclean[Cleanup]
        end
        RS[Room storage (hashmap)\nparticipants, status, created_at]
    end

    FE <--> WS[WebSocket connection] <--> BE

    BE --> MQ
    BE --> CP
    BE --> RS

    note1[[How someone connects:\n- Type in a room code\n- 1–3 participants (FCFS)\n- Display connection status\n- Stream controls available]]

    note2[[Control flow:\nBrowser -> WebSocket -> QUIC bridge\n-> super-quinn QUIC processing\n-> message routing -> room broadcast]]
```