function main() {
  console.log("nicks-cors-test");
  $.ajax({
    url: "http://api.dev.test",
    success: function (data) {
      console.log(data);
    },
  });
}
