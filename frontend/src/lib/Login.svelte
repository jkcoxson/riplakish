<script>
  export let API_URL;
  let username = "";
  let password = "";
  export let loginPopupVisible = false;
  export let fetchRedirects;

  // Function to handle login
  async function login() {
    const res = await fetch(`${API_URL}/admin/login`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        "X-Username": username,
        "X-Password": password,
      },
    });
    if (res.status === 200) {
      // Close the popup
      loginPopupVisible = false;
      await fetchRedirects();
    } else {
      alert("Invalid username or password");
    }
  }
</script>

<div class="popup-content">
  <h2>Login</h2>
  <p>
    This site uses cookies to store your login token. By logging in, you accept
    cookies. Don't worry, they don't have raisins.
  </p>
  <br />
  <!-- Inputs for login -->
  <div class="popup-settings">
    <div class="setting">
      <label for="username">Username:</label>
      <input type="text" id="usn" bind:value={username} />
    </div>
    <div class="setting">
      <label for="password">Password: </label>
      <input type="password" id="pw" bind:value={password} />
    </div>
  </div>
  <br />
  <button on:click={login}>Login</button>
  <br />
</div>

<style>
  /* Styling for the input box */
  input[type="text"] {
    padding: 10px;
    border: 1px solid #4caf50; /* Green border */
    border-radius: 8px; /* Rounded border */
    background-color: #2e3338; /* Dark background */
    color: #fff; /* Text color */
    margin-bottom: 10px;
    width: 80%; /* Set the width */
    max-width: 30vw;
  }

  input[type="password"] {
    padding: 10px;
    border: 1px solid #4caf50; /* Green border */
    border-radius: 8px; /* Rounded border */
    background-color: #2e3338; /* Dark background */
    color: #4caf50; /* Text color */
    margin-bottom: 10px;
    width: 80%; /* Set the width */
    max-width: 30vw;
  }

  .popup-content {
    background-color: #242424;
    padding: 20px;
    max-width: 400px;
    margin: 100px auto;
    border-radius: 8px;
    box-shadow: 0 0 10px rgba(0, 0, 0, 0.3);
  }

  .popup-settings {
    display: grid;
    grid-template-columns: 1fr;
    grid-gap: 10px;
    align-items: center;
  }

  .setting {
    display: grid;
    grid-template-columns: max-content 1fr;
    align-items: center;
  }

  .setting label {
    margin-right: 10px;
  }
</style>
