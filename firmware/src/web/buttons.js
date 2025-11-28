document.addEventListener("DOMContentLoaded", function() {
  const getButton = document.getElementById("get");
  getButton.addEventListener("click", function() {
      fetch('/get')
        .then(response => {
          if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
          }
          return response.json();
        })
        .then(data => {
          console.log(data);
          alert(data.temperature);
        })
        .catch(error => {
          console.error('Fetch error:', error);
        });
  });
  const incrementButton = document.getElementById("increment");
  incrementButton.addEventListener("click", function() {
      fetch('/increment')
        .then(response => {
          if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
          }
          return void 0;
        }).catch(error => {
          console.error('Fetch error:', error);
        });
  });
});