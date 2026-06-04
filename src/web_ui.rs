pub const HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <title>Mi Band Heart Rate</title>
    <style>
        @font-face {
            font-family: 'MiSans VF';
            src: url('https://cdn.jsdelivr.net/npm/misans@4.0/lib/Normal/MiSans-VF.woff2') format('woff2');
            font-weight: 100 900;
        }

        :root {
            --red: #FF3B30;
            --glow: rgba(255, 59, 48, 0.35);
            --white: #FFFFFF;
            --white-dim: rgba(255, 255, 255, 0.25);
        }

        html, body {
            background: transparent !important;
            margin: 0;
            padding: 0;
            overflow: hidden;
            width: 100vw;
            height: 100vh;
        }

        body {
            display: flex;
            align-items: flex-end;
            justify-content: flex-start;
        }

        .container {
            display: flex;
            align-items: center;
            gap: 14px;
            margin-left: 60px;
            margin-bottom: 60px;
        }

        .heart {
            width: 90px;
            height: 90px;
            flex-shrink: 0;
            fill: var(--red);
            animation: pulse 1.2s ease-in-out infinite;
            filter: drop-shadow(0 0 12px var(--glow));
        }

        @keyframes pulse {
            0%   { transform: scale(1);    filter: drop-shadow(0 0 12px var(--glow)); }
            10%  { transform: scale(1.14); filter: drop-shadow(0 0 20px var(--glow)); }
            20%  { transform: scale(1);    filter: drop-shadow(0 0 12px var(--glow)); }
            45%  { transform: scale(1.07); filter: drop-shadow(0 0 16px var(--glow)); }
            60%  { transform: scale(1);    filter: drop-shadow(0 0 12px var(--glow)); }
        }

        .bpm-number {
            font-family: 'MiSans VF', sans-serif;
            font-weight: 700;
            font-size: 100px;
            line-height: 1;
            color: var(--white);
            text-shadow: 0 0 30px rgba(255, 255, 255, 0.3);
            transition: color 0.4s ease;
        }

        .bpm-number.dim {
            color: var(--white-dim);
            text-shadow: none;
        }
    </style>
</head>
<body>
    <div class="container">
        <svg class="heart" viewBox="0 0 24 24">
            <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5
                     2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09
                     C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5
                     c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
        </svg>
        <div class="bpm-number" id="heart-rate">--</div>
    </div>

    <script>
        const el = document.getElementById('heart-rate');

        async function fetchRate() {
            try {
                const res = await fetch('/heart-rate');
                if (!res.ok) {
                    throw new Error(`HTTP error! status: ${res.status}`);
                }
                const data = await res.json();

                const isValidReading = typeof data.heart_rate === 'number' &&
                                       data.heart_rate > 0 &&
                                       data.connected &&
                                       !data.scanning;

                if (isValidReading) {
                    el.textContent = data.heart_rate;
                    el.classList.remove('dim');
                } else {
                    el.textContent = '--';
                    el.classList.add('dim');
                }
            } catch (error) {
                console.error('Failed to fetch heart rate:', error);
                el.textContent = '--';
                el.classList.add('dim');
            }
        }

        setInterval(fetchRate, 1000);
        fetchRate();
    </script>
</body>
</html>"##;