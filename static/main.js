function main() {
  console.log("nicks-cors-test");
  $.ajax({
    url: "http://localhost:8000",
    success: function (data) {
      console.log(data);
    },
  });
}
