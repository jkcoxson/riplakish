<script>
  import { onMount } from "svelte";
  import QRious from "qrious";

  export let popupVisible;
  let qrSize = 100;
  let qrColor = "#000000";
  let qrBackgroundColor = "#ffffff";
  let qrTransparent = false;
  export let qrValue = "";
  export let qrEdit = false;

  onMount(() => {
    renderQRCode();
  });

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
</script>

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
