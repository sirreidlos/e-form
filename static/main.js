function main() {
  console.log("nicks-cors-test");
  $.ajax({
    url: "http://api.dev.test",
    success: function (data) {
      console.log(data);
    },
  });
}

const evtSource = new EventSource(
  "http://api.dev.test/stream/63d1eb130d6b861224602a68"
);

const eventList = document.getElementById("ul");

evtSource.onmessage = (e) => {
  console.log(e);
  const newElement = document.createElement("li");

  newElement.textContent = `message: ${e.data}`;
  eventList.appendChild(newElement);
};
