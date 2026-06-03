pub const HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <title>Band Heart Rate</title>
    <style>
        @font-face {
            font-family: 'MiSans VF';
            src: url('https://cdn.cnbj1.fds.api.mi-img.com/vipmlmodel/font/MiSans/MiSans_VF.woff2') format('woff2-variations');
            font-weight: 100 900;
            font-style: normal;
            font-display: swap;
        }

        :root {
            --red: #FF3B30;
            --glow: rgba(255, 59, 48, 0.35);
        }

        html, body {
            margin: 0;
            padding: 0;
            overflow: hidden;
            width: 100vw;
            height: 100vh;
        }

        body {
            background: #0a0a0a;
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
            will-change: transform, filter;
        }

        @keyframes pulse {
            0%, 30%, 60%, 100% {
                transform: scale(1);
                filter: drop-shadow(0 0 12px var(--glow));
            }
            15% {
                transform: scale(1.14);
                filter: drop-shadow(0 0 20px var(--glow));
            }
            45% {
                transform: scale(1.07);
                filter: drop-shadow(0 0 16px var(--glow));
            }
        }

        .bpm-number {
            font-family: 'MiSans VF', "Segoe UI", "Microsoft YaHei", sans-serif;
            font-weight: 700;
            font-size: 88px;
            line-height: 1;
            color: #ffffff;
            text-shadow: 0 0 30px rgba(255, 255, 255, 0.3);
            font-variant-numeric: tabular-nums;
            font-feature-settings: 'tnum';
            transition: opacity 0.15s ease;
        }

        .bpm-number.updating {
            opacity: 0.6;
        }

        @media (prefers-reduced-motion: reduce) {
            .heart { animation: none; }
            .bpm-number { transition: none; }
        }
    </style>
</head>
<body>
    <div class="container">
        <svg class="heart" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5
                     2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09
                     C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5
                     c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
        </svg>
        <div class="bpm-number" id="heart-rate" role="status" aria-live="polite">--</div>
    </div>

    <script>
        const el = document.getElementById('heart-rate');
        let currentHr = null;

        function connect() {
            const source = new EventSource('/heart-rate-stream');

            source.onmessage = (event) => {
                const data = JSON.parse(event.data);
                const newHr = (data.connected && data.heart_rate > 0)
                    ? data.heart_rate
                    : null;

                if (newHr !== currentHr) {
                    el.classList.add('updating');
                    requestAnimationFrame(() => {
                        requestAnimationFrame(() => {
                            el.textContent = newHr != null ? newHr : '--';
                            el.classList.remove('updating');
                        });
                    });
                    currentHr = newHr;
                }
            };

            source.onerror = () => {
                el.textContent = '--';
                currentHr = null;
                source.close();
                // Retry connection after 3 seconds
                setTimeout(connect, 3000);
            };
        }

        connect();
    </script>
</body>
</html>"##;