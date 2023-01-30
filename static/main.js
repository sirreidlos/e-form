function main() {
  console.log("nicks-cors-test");
  $.ajax({
    url: "http://api.dev.test",
    success: function (data) {
      console.log(data);
    },
  });
}

const eventList = document.getElementById("ul");

fetch("http://api.dev.test/stream/63d1eb130d6b861224602a68", {
  headers: {
    Authorization:
      "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE2NzUwNTI2MDcsInN1YiI6IjYzZDFlMTg0ZjIwYmE5ZmU1MjA4ZjJjMiJ9.o9yni8zKF15vDBqKLDEZahaJRYWj7pmEs700lmx10VA",
  },
}).then((response) => {
  var evtSource = new EventSource(response.url);
  evtSource.onmessage = (e) => {
    console.log(e);
    const newElement = document.createElement("li");

    newElement.textContent = `message: ${e.data}`;
    eventList.appendChild(newElement);
  };
});

// fetch("http://api.dev.test/stream/63d1eb130d6b861224602a68", {
//   headers: {
//     Authorization:
//       "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE2NzUwNTI2MDcsInN1YiI6IjYzZDFlMTg0ZjIwYmE5ZmU1MjA4ZjJjMiJ9.o9yni8zKF15vDBqKLDEZahaJRYWj7pmEs700lmx10VA",
//   },
// }).then((res) => {
//   var evtSource = new EventSource(res.url);
//   evtSource.onmessage = (e) => {
//     console.log(e);
//     const newElement = document.createElement("li");

//     newElement.textContent = `message: ${e.data}`;
//     eventList.appendChild(newElement);
//   };
// });

// const evtSource = new EventSource(
//   "http://api.dev.test/stream/63d1eb130d6b861224602a68"
// );
