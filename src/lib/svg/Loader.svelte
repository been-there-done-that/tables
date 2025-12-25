<div>
  <div class="loader-container">
  <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <style>
      .liquid-sq {
        fill: #3b82f6;
        opacity: 0.8;
        transform-origin: center;
        animation: morph 2s infinite ease-in-out alternate;
      }

      .container-group {
        transform-origin: center;
        animation: spin 8s infinite linear;
      }

      /* Delays to create a wave pattern */
      .sq1 { animation-delay: 0.0s; fill: #60a5fa; }
      .sq2 { animation-delay: 0.2s; fill: #3b82f6; }
      .sq3 { animation-delay: 0.4s; fill: #2563eb; }
      .sq4 { animation-delay: 0.6s; fill: #1d4ed8; }

      @keyframes morph {
        0% { transform: scale(0.8); rx: 2px; }
        50% { transform: scale(1.1); rx: 12px; } /* Becomes circle-ish */
        100% { transform: scale(0.8); rx: 2px; }
      }
      
      @keyframes spin {
        100% { transform: rotate(360deg); }
      }
    </style>

    <g class="container-group">
      <rect class="liquid-sq sq1" x="20" y="20" width="25" height="25" />
      <rect class="liquid-sq sq2" x="55" y="20" width="25" height="25" />
      <rect class="liquid-sq sq3" x="55" y="55" width="25" height="25" />
      <rect class="liquid-sq sq4" x="20" y="55" width="25" height="25" />
    </g>
  </svg>
</div>
</div>