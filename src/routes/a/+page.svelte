<div>
  <div class="loader-container">
  <svg width="120" height="120" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <style>
      .squiggle {
        fill: none;
        stroke: #3b82f6;
        stroke-width: 4;
        stroke-linecap: round;
        stroke-linejoin: round;
        stroke-dasharray: 200;
        stroke-dashoffset: 200;
        animation: draw 2s ease-in-out infinite alternate;
      }
      
      @keyframes draw {
        0% { stroke-dashoffset: 200; stroke: #94a3b8; }
        50% { stroke: #3b82f6; }
        100% { stroke-dashoffset: 0; stroke: #2563eb; }
      }
    </style>
    
    <path class="squiggle" d="M15,50 C15,50 25,20 35,50 C45,80 55,20 65,50 C75,80 85,50 85,50" />
    
    <circle cx="85" cy="50" r="3" fill="#3b82f6">
      <animate attributeName="opacity" values="0;0;1" dur="2s" repeatCount="indefinite" />
    </circle>
    <circle cx="95" cy="50" r="3" fill="#3b82f6">
      <animate attributeName="opacity" values="0;0;0;1" dur="2s" repeatCount="indefinite" />
    </circle>
  </svg>
</div>
</div>


<div>
  <div class="loader-container">
  <svg width="120" height="120" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <style>
      .center-star { fill: #3b82f6; animation: pulse 2s infinite; }
      .orbit-dot { fill: #64748b; }
      
      .orbit-path-1 { animation: orbit1 3s infinite linear; transform-origin: 50px 50px; }
      .orbit-path-2 { animation: orbit2 4s infinite linear; transform-origin: 50px 50px; }
      .orbit-path-3 { animation: orbit3 2.5s infinite linear; transform-origin: 50px 50px; }

      @keyframes pulse {
        0%, 100% { r: 6; opacity: 1; }
        50% { r: 8; opacity: 0.6; }
      }
      
      @keyframes orbit1 { from { transform: rotate(0deg) translateX(25px) rotate(0deg); } to { transform: rotate(360deg) translateX(25px) rotate(-360deg); } }
      @keyframes orbit2 { from { transform: rotate(120deg) translateX(35px) rotate(-120deg); } to { transform: rotate(480deg) translateX(35px) rotate(-480deg); } }
      @keyframes orbit3 { from { transform: rotate(240deg) translateX(20px) rotate(-240deg); } to { transform: rotate(-120deg) translateX(20px) rotate(120deg); } }
    </style>

    <circle class="center-star" cx="50" cy="50" r="6" />

    <g class="orbit-path-1"><circle class="orbit-dot" cx="50" cy="50" r="3" /></g>
    <g class="orbit-path-2"><circle class="orbit-dot" cx="50" cy="50" r="2" /></g>
    <g class="orbit-path-3"><circle class="orbit-dot" cx="50" cy="50" r="4" /></g>
    
    <circle cx="50" cy="50" r="25" stroke="#e2e8f0" stroke-width="1" fill="none" stroke-dasharray="4 4" />
    <circle cx="50" cy="50" r="35" stroke="#e2e8f0" stroke-width="1" fill="none" stroke-dasharray="2 6" />
  </svg>
</div>
</div>

<div>
  <div class="loader-container">
  <svg width="120" height="120" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <defs>
       <filter id="scribble">
        <feTurbulence type="fractalNoise" baseFrequency="0.02" numOctaves="3" result="noise" />
        <feDisplacementMap in="SourceGraphic" in2="noise" scale="3" />
      </filter>
    </defs>

    <style>
      .morph-path {
        fill: none;
        stroke: #3b82f6;
        stroke-width: 3;
        stroke-linecap: round;
        stroke-linejoin: round;
        filter: url(#scribble); /* Apply the doodle effect */
        /* The morphing magic happens here */
        animation: rotate 6s linear infinite, color-change 6s infinite;
      }
      
      /* Rotates the whole shape slowly */
      @keyframes rotate {
        0% { transform: rotate(0deg); transform-origin: 50px 50px; }
        100% { transform: rotate(360deg); transform-origin: 50px 50px; }
      }
      
      @keyframes color-change {
        0%, 100% { stroke: #3b82f6; } /* Blue */
        33% { stroke: #10b981; } /* Emerald */
        66% { stroke: #f59e0b; } /* Amber */
      }
    </style>

    <path class="morph-path" d="M50,15 L85,80 L15,80 Z">
      <animate 
        attributeName="d" 
        dur="3s" 
        repeatCount="indefinite" 
        values="
          M50,15 L85,80 L15,80 Z;                 /* Triangle */
          M15,15 L85,15 L85,85 L15,85 Z;          /* Square */
          M50,10 C20,10 10,20 10,50 C10,80 20,90 50,90 C80,90 90,80 90,50 C90,20 80,10 50,10 Z; /* Circle */
          M50,15 L85,80 L15,80 Z                  /* Back to Triangle */
        "
      />
    </path>
  </svg>
</div>
</div>

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

<div>
  <div class="loader-container">
  <svg width="120" height="120" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <style>
      .row {
        height: 15px;
        rx: 5px; /* Rounded corners for friendly UI */
        fill: #cbd5e1;
        animation: slideIn 1.5s infinite ease-in-out;
      }
      
      .row-header {
        fill: #3b82f6; /* Header is Blue */
        width: 80px;
        animation-delay: 0s;
      }

      /* Staggered widths and delays for "random data" feel */
      .r1 { width: 60px; animation-delay: 0.1s; fill: #94a3b8; }
      .r2 { width: 70px; animation-delay: 0.2s; fill: #cbd5e1; }
      .r3 { width: 50px; animation-delay: 0.3s; fill: #e2e8f0; }

      @keyframes slideIn {
        0% { transform: translateX(-10px); opacity: 0; width: 10px; }
        40% { transform: translateX(0); opacity: 1; } /* Overshoot handled by natural width change */
        80% { opacity: 1; transform: translateX(0); }
        100% { opacity: 0; transform: translateX(10px); }
      }
    </style>

    <rect class="row row-header" x="10" y="15" />
    
    <rect class="row r1" x="10" y="35" />
    <rect class="row r2" x="10" y="55" />
    <rect class="row r3" x="10" y="75" />
  </svg>
</div>
</div>

<div>
  <div class="loader-container">
  <svg width="120" height="120" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <filter id="displacementFilter">
        <feTurbulence type="fractalNoise" baseFrequency="0.05" numOctaves="2" result="noise" />
        <feDisplacementMap in="SourceGraphic" in2="noise" scale="3" xChannelSelector="R" yChannelSelector="G" />
      </filter>
    </defs>
    
    <style>
      .doodle-rect {
        fill: #e2e8f0; /* Light Gray */
        stroke: #475569; /* Pencil Gray outline */
        stroke-width: 2;
        /* Apply the sketch filter */
        filter: url(#displacementFilter);
        transform-origin: center;
        animation: sketch-pop 1.8s infinite cubic-bezier(0.25, 1, 0.5, 1);
      }

      /* Staggered delays */
      .doodle-rect:nth-child(2) { animation-delay: 0.1s; }
      .doodle-rect:nth-child(3) { animation-delay: 0.2s; }
      .doodle-rect:nth-child(4) { animation-delay: 0.3s; }
      .doodle-rect:nth-child(5) { animation-delay: 0.4s; fill: #3b82f6; stroke: #1d4ed8; } /* Blue Center */
      .doodle-rect:nth-child(6) { animation-delay: 0.5s; }
      .doodle-rect:nth-child(7) { animation-delay: 0.6s; }
      .doodle-rect:nth-child(8) { animation-delay: 0.7s; }
      .doodle-rect:nth-child(9) { animation-delay: 0.8s; }
      .doodle-rect:nth-child(10) { animation-delay: 0.9s; }

      @keyframes sketch-pop {
        0%, 100% { transform: scale(0.9); }
        50% { transform: scale(1.05) rotate(2deg); } /* Subtle rotation adds to the doodle feel */
      }
    </style>

    <g transform="translate(5,5)">
      <rect class="doodle-rect" x="0" y="0" width="25" height="25" />
      <rect class="doodle-rect" x="30" y="0" width="25" height="25" />
      <rect class="doodle-rect" x="60" y="0" width="25" height="25" />
      
      <rect class="doodle-rect" x="0" y="30" width="25" height="25" />
      <rect class="doodle-rect" x="30" y="30" width="25" height="25" />
      <rect class="doodle-rect" x="60" y="30" width="25" height="25" />
      
      <rect class="doodle-rect" x="0" y="60" width="25" height="25" />
      <rect class="doodle-rect" x="30" y="60" width="25" height="25" />
      <rect class="doodle-rect" x="60" y="60" width="25" height="25" />
    </g>
  </svg>
</div>
</div>
<div>
    <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <style>
    .sq { fill: #cbd5e1; animation: flip 2s infinite ease-in-out; }
    .sq:nth-child(1) { animation-delay: 0.0s; }
    .sq:nth-child(2) { animation-delay: 0.1s; }
    .sq:nth-child(3) { animation-delay: 0.2s; }
    .sq:nth-child(4) { animation-delay: 0.3s; }
    .sq:nth-child(5) { animation-delay: 0.4s; fill: #3b82f6; } /* Center highlighted */
    .sq:nth-child(6) { animation-delay: 0.5s; }
    .sq:nth-child(7) { animation-delay: 0.6s; }
    .sq:nth-child(8) { animation-delay: 0.7s; }
    .sq:nth-child(9) { animation-delay: 0.8s; }

    @keyframes flip {
      0% { transform: perspective(120px) rotateX(0deg) rotateY(0deg); }
      50% { transform: perspective(120px) rotateX(-180.1deg) rotateY(0deg); fill: #3b82f6; }
      100% { transform: perspective(120px) rotateX(-180deg) rotateY(-179.9deg); }
    }
  </style>
  
  <rect class="sq" x="10" y="10" width="20" height="20" rx="2" />
  <rect class="sq" x="40" y="10" width="20" height="20" rx="2" />
  <rect class="sq" x="70" y="10" width="20" height="20" rx="2" />
  
  <rect class="sq" x="10" y="40" width="20" height="20" rx="2" />
  <rect class="sq" x="40" y="40" width="20" height="20" rx="2" />
  <rect class="sq" x="70" y="40" width="20" height="20" rx="2" />
  
  <rect class="sq" x="10" y="70" width="20" height="20" rx="2" />
  <rect class="sq" x="40" y="70" width="20" height="20" rx="2" />
  <rect class="sq" x="70" y="70" width="20" height="20" rx="2" />
</svg>
</div>

<div class="loader-container">
  <div class="table-frame">
    <div class="row header">
      <div class="cell" style="--delay: 0s"></div>
      <div class="cell" style="--delay: 0.1s"></div>
      <div class="cell" style="--delay: 0.2s"></div>
    </div>
    <div class="row">
      <div class="cell data" style="--delay: 0.3s"></div>
      <div class="cell data" style="--delay: 0.4s"></div>
      <div class="cell data" style="--delay: 0.5s"></div>
    </div>
    <div class="row">
      <div class="cell data" style="--delay: 0.6s"></div>
      <div class="cell data" style="--delay: 0.7s"></div>
      <div class="cell data" style="--delay: 0.8s"></div>
    </div>
  </div>
</div>

<style>
  .loader-container {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    width: 100%;
  }

  .table-frame {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
    border: 2px solid #e5e7eb;
    border-radius: 6px;
    background: #fff;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
  }

  .row {
    display: flex;
    gap: 4px;
  }

  .cell {
    width: 30px;
    height: 10px;
    background-color: #e5e7eb;
    border-radius: 2px;
    animation: pulse 1.5s infinite ease-in-out;
    animation-delay: var(--delay);
  }

  .header .cell {
    height: 12px;
    background-color: #9ca3af; /* Darker for header */
  }

  /* Animation to simulate data "fetching" */
  @keyframes pulse {
    0%, 100% { opacity: 0.4; transform: scale(0.95); }
    50% { opacity: 1; transform: scale(1); background-color: #3b82f6; }
  }
</style>