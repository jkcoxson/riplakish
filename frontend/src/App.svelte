<!-- App.svelte -->
<script>
  import { onMount } from "svelte";
  import QRious from "qrious";

  const API_URL = "http://10.7.0.6:3009"; // change for npm run dev
  let BASE_URL = "127.0.0.1";

  let redirects = [];
  let selectedRedirect = null;
  let newRedirectUrl = "";
  let logEvents = [];
  let staticQr = "";

  let loginPopupVisible = false;
  let username = "";
  let password = "";

  // Check if X-Token cookie is set
  function isLoggedIn() {
    return document.cookie.includes("X-Token");
  }

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

  if (!isLoggedIn()) {
    loginPopupVisible = true;
  }

  // Fetch redirects stats from /admin/stats API endpoint
  async function fetchRedirects() {
    const res = await fetch(`${API_URL}/admin/stats`);
    if (res.status === 401) {
      // Handle unauthorized access
      loginPopupVisible = true;
      return;
    }
    redirects = await res.json();
  }

  // Fetch the base URL
  async function fetchBaseUrl() {
    const res = await fetch(`${API_URL}/base`);
    BASE_URL = await res.text();
  }

  // Fetch details of a specific redirect and its log events
  async function fetchRedirectDetails(code) {
    selectedRedirect = code;

    // Fetch log events for the selected redirect
    const logRes = await fetch(`${API_URL}/admin/logs/${code}`);
    if (logRes.status === 401) {
      // Handle unauthorized access
      loginPopupVisible = true;
      return;
    }
    logEvents = await logRes.json();
  }

  // Modify the URL of a redirect
  async function modifyRedirect(code, newUrl) {
    await fetch(`${API_URL}/admin/modify/${code}/${newUrl}`, {
      method: "POST",
    });
    // Update the UI or perform any necessary actions after modifying
    fetchRedirects();
  }

  async function modifyComment(code, newComment) {
    await fetch(`${API_URL}/admin/modify-comment/${code}/${newComment}`, {
      method: "POST",
    });
    // Update the UI or perform any necessary actions after modifying
    fetchRedirects();
  }

  // Delete a redirect
  async function removeRedirect(url) {
    await fetch(`${API_URL}/admin/remove/${url}`, { method: "DELETE" });
    // Update the UI or perform any necessary actions after removing
    fetchRedirects();
  }

  // Create a new redirect
  async function addRedirect() {
    await fetch(`${API_URL}/admin/add/${newRedirectUrl}`, { method: "POST" });
    // Update the UI or perform any necessary actions after adding
    fetchRedirects();
  }

  let popupVisible = false;
  let qrSize = 100;
  let qrColor = "#000000";
  let qrBackgroundColor = "#ffffff";
  let qrTransparent = false;
  let qrValue = "";
  let qrEdit = false;

  // Function to open the QR code settings popup
  function openPopup(url, edit) {
    qrValue = url;
    qrEdit = edit;
    popupVisible = true;
    renderQRCode();
  }

  // Function to customize QR code settings and download
  function customizeAndDownload() {
    const qrCanvas = document.getElementById("qr-canvas");
    const qr = new QRious({
      value: qrValue,
      foreground: qrColor,
      background: qrBackgroundColor,
      backgroundAlpha: qrTransparent ? 0 : 1,
      size: qrSize,
    });

    // Download the customized QR code
    const link = document.createElement("a");
    link.href = qr.toDataURL();
    link.download = `redirect_qr.png`;
    link.click();

    popupVisible = false; // Close the popup after downloading
  }

  function renderQRCode() {
    const qrCanvas = document.getElementById("qr-canvas");
    const qr = new QRious({
      value: qrValue,
      foreground: qrColor,
      background: qrBackgroundColor,
      backgroundAlpha: qrTransparent ? 0 : 1,
      size: qrSize,
      element: qrCanvas,
    });
  }

  // Fetch redirects stats when the component mounts
  onMount(() => {
    fetchRedirects();
    fetchBaseUrl();
  });
</script>

<main class="container">
  {#if selectedRedirect}
    <div class="redirect-details">
      <!-- Details of a selected redirect -->
      <h2>Redirect Details</h2>
      <div class="redirect-box">
        <p>Code: {selectedRedirect}</p>
      </div>
      <button on:click={() => (selectedRedirect = null)}>Close Details</button>
    </div>

    <!-- Log events for the selected redirect -->
    <div class="log-events">
      <h3>Log Events</h3>
      <table class="log-table">
        <thead>
          <tr>
            <th>Timestamp</th>
            <th>IP</th>
            <th>URL</th>
          </tr>
        </thead>
        <tbody>
          {#each logEvents as logEvent}
            <tr>
              <td>{logEvent.timestamp}</td>
              <td>{logEvent.ip}</td>
              <td>{logEvent.url}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <!-- Redirect Stats -->
    <div class="redirect-stats">
      <h1>Redirects for {BASE_URL}:</h1>
      <ul>
        {#each redirects as redirect}
          <li>
            <div class="redirect-box">
              <p>Code: {redirect.code}</p>
              <p>Visits: {redirect.visits}</p>
              <!-- Display URL as an input field for easy modification -->
              <input
                type="text"
                bind:value={redirect.url}
                placeholder="Enter URL"
                on:change={(event) =>
                  modifyRedirect(redirect.code, redirect.url)}
              />
            </div>
            <div class="redirect-actions">
              <button on:click={() => fetchRedirectDetails(redirect.code)}
                >View Details</button
              >
              <button on:click={() => removeRedirect(redirect.code)}
                >Delete</button
              >
              <button
                on:click={() =>
                  openPopup(`${BASE_URL}/r/${redirect.code}`, false)}
                >QR Code</button
              >
              <hr />
              <input
                type="text"
                bind:value={redirect.comment}
                placeholder="Comment..."
                on:change={(event) =>
                  modifyComment(redirect.code, redirect.comment)}
              />
            </div>
          </li>
        {/each}
      </ul>
    </div>

    <!-- Create New Redirect -->
    <div class="create-redirect">
      <hr />
      <h2>Create New Redirect</h2>
      <p>
        The destination of this QR code can change in the future, and visits
        will be tracked. This QR code will not work if Riplakish is not running.
      </p>
      <input type="text" bind:value={newRedirectUrl} placeholder="Enter URL" />
      <button on:click={addRedirect}>Add Redirect</button>
    </div>

    <!-- Static QR-->
    <div class="static-qr">
      <hr />
      <h2>Create Static Code</h2>
      <p>
        This encodes the URL itself. You CANNOT change where this QR code
        "points". Scans will not be tracked.
      </p>
      <input type="text" bind:value={staticQr} placeholder="Enter URL" />
      <button on:click={() => openPopup(staticQr, true)}>Generate</button>
    </div>
  {/if}

  <div class="popup" style="display: {loginPopupVisible ? 'block' : 'none'}">
    <div class="popup-content">
      <h2>Login</h2>
      <br />
      <!-- Inputs for login -->
      <div class="popup-settings">
        <div class="setting">
          <label for="username">Username:</label>
          <input type="text" id="usn" bind:value={username} />
        </div>
        <div class="setting">
          <label for="password">Password: </label>
          <input type="text" id="pw" bind:value={password} />
        </div>
      </div>
      <br />
      <button on:click={login}>Login</button>
      <br />
      <canvas id="qr-canvas"></canvas>
    </div>
  </div>

  <div class="popup" style="display: {popupVisible ? 'block' : 'none'}">
    <div class="popup-content">
      <h2>Customize QR Code</h2>
      <br />
      <!-- Include input fields or sliders to configure QR code properties -->
      <div class="popup-settings">
        <div class="setting">
          <label for="qrSize">Size:</label>
          <input
            type="number"
            id="qrSize"
            bind:value={qrSize}
            on:change={renderQRCode}
          />
        </div>
        <div class="setting">
          <label for="qrColor">Color:</label>
          <input
            type="color"
            id="qrColor"
            bind:value={qrColor}
            on:change={renderQRCode}
          />
        </div>
        <div class="setting">
          <label for="qrBackgroundColor">Background Color:</label>
          <input
            type="color"
            id="qrBackgroundColor"
            bind:value={qrBackgroundColor}
            on:change={renderQRCode}
          />
        </div>
        <div class="setting">
          <label for="qrTransparent">Transparent Background:</label>
          <input
            type="checkbox"
            id="qrTransparent"
            bind:checked={qrTransparent}
            on:change={renderQRCode}
          />
        </div>
        {#if qrEdit}
          <div class="setting">
            <label for="qrValue">QR Code Value:</label>
            <input
              type="text"
              id="qrValue"
              bind:value={qrValue}
              on:input={renderQRCode}
            />
          </div>
        {/if}
      </div>
      <!-- Add more customizable options for QR code -->
      <br />
      <button on:click={customizeAndDownload}>Download QR Code</button>
      <button on:click={() => (popupVisible = false)}>Close</button>
      <br />
      <canvas id="qr-canvas"></canvas>
    </div>
  </div>
</main>

<style>
  .container {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin: 20px;
    color: #fff; /* Set text color to white */
  }

  .redirect-details {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 20px;
  }

  .redirect-box {
    border-radius: 8px; /* Rounded border */
    padding: 10px;
    margin-bottom: 10px;
    background-color: #2e3338; /* Dark background */
  }

  .redirect-stats ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .redirect-stats li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border: 1px solid #4caf50; /* Green border */
    border-radius: 8px; /* Rounded border */
    padding: 10px;
    margin-bottom: 10px;
    background-color: #2e3338; /* Dark background */
    box-shadow: 0 2px 4px -1px rgba(0, 0, 0, 0.2);
  }

  .redirect-actions button {
    margin-left: 10px;
  }

  .log-events {
    width: 100%;
  }

  .log-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 20px;
  }

  .log-table th,
  .log-table td {
    border-radius: 8px; /* Rounded border */
    padding: 8px;
    text-align: left;
    background-color: #2e3338; /* Dark background */
  }

  .log-table th {
    background-color: #4caf50; /* Green background for headers */
    font-weight: bold;
    color: #fff; /* White text color */
  }

  .create-redirect {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 20px;
  }

  .create-redirect {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 20px;
  }

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

  /* Styling for the input placeholder text */
  .create-redirect input[type="text"]::placeholder {
    color: #ccc; /* Placeholder text color */
  }

  .create-redirect button {
    padding: 10px;
    border: none;
    border-radius: 8px;
    background-color: #4caf50; /* Green background */
    color: #fff; /* Text color */
    cursor: pointer;
    transition: background-color 0.3s ease;
    width: 150px; /* Set the width */
  }

  .create-redirect button:hover {
    background-color: #45a049; /* Darker green on hover */
  }

  /* Styling for the Popup Modal */
  .popup {
    display: none;
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5); /* Semi-transparent background */
    z-index: 999;
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

  canvas {
    margin: 10px;
  }
</style>
