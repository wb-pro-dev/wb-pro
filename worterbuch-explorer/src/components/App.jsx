import * as React from "react";
import { decode_server_message, encode_client_message } from "worterbuch-wasm";
import TopicTree from "./TopicTree";
import SortedMap from "collections/sorted-map";

export default function App() {
  const loc = window.location;
  let proto;
  if (loc.protocol === "https:") {
    proto = "wss";
  } else {
    proto = "ws";
  }
  const url = `${proto}://${loc.hostname}:${loc.port}/ws`;

  const dataRef = React.useRef(new SortedMap());
  const [data, setData] = React.useState(new SortedMap());
  const [socket, setSocket] = React.useState();
  const [multiWildcard, setMultiWildcard] = React.useState();
  const separatorRef = React.useRef();

  React.useEffect(() => {
    const topic = multiWildcard;
    if (topic && socket) {
      dataRef.current = new SortedMap();
      const subscrMsg = {
        pSubscribe: { transactionId: 1, requestPattern: topic, unique: true },
      };
      const buf = encode_client_message(subscrMsg);
      socket.send(buf);
    }
  }, [multiWildcard, socket]);

  React.useEffect(() => {
    console.log("Connecting to server.");
    const socket = new WebSocket(url);
    socket.onclose = (e) => {
      setSocket(undefined);
      setMultiWildcard(undefined);
    };
    socket.onmessage = async (e) => {
      const buf = await e.data.arrayBuffer();
      const uint8View = new Uint8Array(buf);
      const msg = decode_server_message(uint8View);
      if (msg.pState) {
        mergeKeyValuePairs(
          msg.pState.keyValuePairs,
          dataRef.current,
          separatorRef.current
        );
        setData(new SortedMap(dataRef.current));
      }
      if (msg.handshake) {
        separatorRef.current = msg.handshake.separator;
        setMultiWildcard(msg.handshake.multiWildcard);
      }
    };
    socket.onopen = () => {
      console.log("Connected to server.");
      setSocket(socket);
    };
    return () => {
      console.log("Disconnecting from server.");
      socket.close();
    };
  }, [url]);

  return (
    <div className="App">
      {<TopicTree data={data} separator={separatorRef.current} />}
    </div>
  );
}

function mergeKeyValuePairs(kvps, data, separator) {
  for (const { key, value } of kvps) {
    const segments = key.split(separator);
    mergeIn(data, segments.shift(), segments, value);
  }
}

function mergeIn(data, headSegment, segmentsTail, value) {
  let child = data.get(headSegment);
  if (!child) {
    child = {};
    data.set(headSegment, child);
  } else {
  }

  if (segmentsTail.length === 0) {
    child.value = value;
  } else {
    if (!child.children) {
      child.children = new SortedMap();
    }
    mergeIn(child.children, segmentsTail.shift(), segmentsTail, value);
  }
}
