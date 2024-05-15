<script lang="ts">
	export let name: keyof typeof icons;
	export let width = '1rem';
	export let height = '1rem';
	export let focusable: string | number | null | undefined = undefined;
	let icons = {
		virtual_network_icon: {
            box: 80,
            svg: `<path fill-rule="evenodd" clip-rule="evenodd" d="M36.876 26.894C35.359 22.6245 34.3765 18.102 34.0088 13.4072C35.9352 12.9752 37.9399 12.747 39.9998 12.747C45.6915 12.747 50.9716 14.4899 55.3412 17.4718C54.4371 21.3027 53.1007 24.9669 51.3863 28.4099C50.2427 28.5166 49.0838 28.5711 47.9119 28.5711C44.0675 28.5711 40.3607 27.9839 36.876 26.894ZM55.5693 27.7764C56.6955 25.2352 57.6332 22.5917 58.3645 19.8637C58.9953 20.4393 59.5989 21.0441 60.1732 21.676C61.0067 20.8068 62.0929 20.1818 63.3137 19.9189C61.838 18.2071 60.1771 16.6598 58.3621 15.308C53.2366 11.4905 46.8796 9.23047 39.9998 9.23047C37.1213 9.23047 34.3324 9.62621 31.6858 10.3673C28.5699 11.2399 25.6547 12.5901 23.0239 14.3334C23.8995 15.1241 24.5467 16.163 24.8546 17.3392C26.6235 16.1544 28.5383 15.1716 30.566 14.4234C30.9081 18.1604 31.6155 21.7915 32.6522 25.2807C29.846 24.0054 27.2275 22.3891 24.8493 20.4845C24.5356 21.6667 23.8788 22.7093 22.992 23.4991C26.3335 26.1169 30.0979 28.2207 34.1656 29.6908C35.9432 34.2425 38.2964 38.5057 41.1355 42.391C38.4378 44.997 35.4533 47.3082 32.2342 49.2724C26.7334 42.19 23.059 33.6192 21.9169 24.2672C21.0257 24.7693 19.9967 25.0557 18.9009 25.0557C18.7559 25.0557 18.6121 25.0507 18.4696 25.0409C19.7148 34.7258 23.5017 43.614 29.1391 51.0158C25.4858 52.9089 21.5745 54.3729 17.4719 55.3411C14.4901 50.9715 12.7471 45.6914 12.7471 39.9997C12.7471 34.3952 14.4376 29.1871 17.3384 24.8556C16.1623 24.5478 15.1234 23.9007 14.3327 23.0252C11.1092 27.8903 9.23059 33.7264 9.23059 39.9997C9.23059 46.8795 11.4906 53.2365 15.3081 58.362C20.9139 65.8885 29.8871 70.7689 39.9998 70.7689C41.498 70.7689 42.9716 70.6617 44.4133 70.4545C43.8442 69.5219 43.5163 68.426 43.5163 67.2535C43.5163 67.1778 43.5177 67.1024 43.5204 67.0273C42.3682 67.1758 41.1932 67.2524 39.9998 67.2524C32.0246 67.2524 24.8495 63.8282 19.8637 58.3644C23.9167 57.2779 27.7832 55.736 31.405 53.7969C35.0648 58.0001 39.3584 61.6367 44.1365 64.558C44.6739 63.4569 45.5302 62.5401 46.5851 61.9277C42.0637 59.2503 37.9917 55.8946 34.5093 52.0008C37.6745 50.0277 40.6226 47.7389 43.3094 45.1785C47.8796 50.6752 53.4767 55.2871 59.8056 58.7199C58.1776 60.4417 56.3279 61.9511 54.3025 63.2024C55.0785 64.089 55.6041 65.2004 55.7688 66.4261C58.9107 64.5476 61.6885 62.1252 63.9729 59.2895C68.2235 54.0133 70.7691 47.3014 70.7691 39.9997C70.7691 36.5392 70.1972 33.2101 69.1424 30.1032C68.3251 30.9904 67.2503 31.6368 66.0366 31.9237C66.8268 34.4746 67.2526 37.1865 67.2526 39.9997C67.2526 45.9606 65.3406 51.4709 62.0954 55.9563C55.8025 52.65 50.2599 48.1101 45.7887 42.6587C48.838 39.3526 51.4834 35.6683 53.6466 31.6839C55.8863 31.3658 58.0674 30.8642 60.174 30.1946C59.3496 29.3353 58.7721 28.2372 58.5557 27.0144C57.5768 27.3085 56.5808 27.5632 55.5693 27.7764ZM43.6171 39.821C41.651 37.0638 39.9437 34.1095 38.5305 30.9935C41.5414 31.7089 44.6825 32.0876 47.9119 32.0876C48.4024 32.0876 48.891 32.0789 49.3773 32.0615C47.7024 34.8311 45.7714 37.4285 43.6171 39.821Z" fill="#F4333F"/>
<circle cx="18.9767" cy="18.9735" r="3.51648" fill="#F4333F"/>
<circle cx="64.6318" cy="26.0009" r="3.51648" fill="#F4333F"/>
<circle cx="49.678" cy="67.2509" r="3.51648" fill="#F4333F"/>`
        },
        info_icon: {
            box: 20,
            svg: `<g clip-path="url(#clip0_3416_349)">
<path d="M3.33325 3.33398H16.6666V15.0007C16.6666 15.9211 15.9204 16.6673 14.9999 16.6673H4.99992C4.07944 16.6673 3.33325 15.9211 3.33325 15.0006V3.33398Z" stroke="#D4D5D7" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
<rect x="10" y="6.66602" width="0.01" height="0.01" stroke="#D4D5D7" stroke-width="2.5" stroke-linejoin="round"/>
<path d="M10 10V13.3333" stroke="#D4D5D7" stroke-width="1.66667" stroke-linecap="round" stroke-linejoin="round"/>
</g>
<defs>
<clipPath id="clip0_3416_349">
<rect width="20" height="20" fill="white"/>
</clipPath>
</defs>`
        },
		back_icon: {
			box: 34,
			svg: `	<path
		d="M19.8234 7.87891L10.53 15.8449C9.82226 16.4515 9.82226 17.5463 10.53 18.153L19.8234 26.1189"
		stroke="#FAFBFC"
		stroke-width="3"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>`
		},
        next_icon: {
            box: 34,
            svg: `<path
		d="M14.1762 26.1211L23.4697 18.1551C24.1774 17.5485 24.1774 16.4537 23.4697 15.847L14.1762 7.88109"
		stroke="#3283E8"
		stroke-width="3"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>`
        },
		settings_icon: {
			box: 32,
			svg: `<g clip-path="url(#clip0_2225_2247)">
<ellipse cx="16.0002" cy="15.9987" rx="2.66674" ry="2.66667" stroke="#FAFBFC" stroke-width="2.5"/>
<path d="M6.76572 7.51659C5.45143 8.93312 4.49708 10.6142 3.94139 12.4152L6.44288 13.8594C8.22072 14.8858 8.22074 17.4518 6.44292 18.4782L3.93897 19.9238C4.21493 20.8144 4.59372 21.6883 5.07958 22.5299C5.56542 23.3713 6.13288 24.1363 6.76619 24.8206L9.27018 23.3749C11.048 22.3485 13.2703 23.6316 13.2703 25.6844L13.2703 28.5729C15.1079 28.9922 17.0409 29.0062 18.9249 28.5763L18.9249 25.6844C18.9249 23.6316 21.1471 22.3486 22.9249 23.375L25.4294 24.821C26.7437 23.4044 27.6981 21.7234 28.2538 19.9224L25.7522 18.4781C23.9744 17.4517 23.9743 14.8857 25.7522 13.8593L28.2562 12.4137C27.9802 11.5231 27.6014 10.6492 27.1156 9.8077C26.6297 8.96619 26.0623 8.20118 25.4289 7.5169L22.925 8.96253C21.1471 9.98893 18.9249 8.7059 18.9249 6.65308L18.9249 3.76467C17.0873 3.3454 15.1542 3.33135 13.2703 3.76127L13.2703 6.65315C13.2703 8.70594 11.048 9.98892 9.27022 8.96252L6.76572 7.51659Z" stroke="#FAFBFC" stroke-width="3" stroke-linejoin="round"/>
</g>
<defs>
<clipPath id="clip0_2225_2247">
<rect width="32.0009" height="32" fill="white" transform="translate(-0.000244141)"/>
</clipPath>
</defs>`
		},
		refresh_icon: {
			box: 32,
			svg: `<path d="M18.9714 5.63749C24.6942 7.27848 28.0031 13.248 26.3621 18.9708C24.7212 24.6936 18.7516 28.0026 13.0288 26.3616C7.30601 24.7206 3.99705 18.7511 5.63803 13.0283C6.002 11.759 6.5789 10.6084 7.31882 9.60429M18.5003 9.00083L17.1865 4.87649L21.0003 3.50083" stroke="white" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>`
		},
		exit_icon: {
			box: 32,
			svg: `<g clip-path="url(#clip0_3356_579)">
<path d="M23.3327 22L28.666 15.9999M28.666 15.9999L23.3327 10M28.666 15.9999L12.666 15.9999" stroke="white" stroke-width="2.66667" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M15.333 5.33594L8.66634 5.33594C7.19358 5.33594 5.99967 6.52984 5.99967 8.0026L5.99967 24.0026C5.99967 25.4754 7.19358 26.6693 8.66634 26.6693L15.333 26.6693" stroke="white" stroke-width="2.66667" stroke-linecap="round" stroke-linejoin="round"/>
</g>
<defs>
<clipPath id="clip0_3356_579">
<rect width="32" height="32" fill="white"/>
</clipPath>
</defs>`,
		},
		active_status_icon : {
			box: 38,
			svg: `<g filter="url(#filter0_f_3308_322)">
<ellipse cx="18.4995" cy="19.0021" rx="7.875" ry="7.87511" stroke="#00AA13" stroke-width="2.66665" stroke-linecap="round" stroke-linejoin="round"/>
</g>
<ellipse cx="18.5" cy="19.002" rx="5.25" ry="5.25007" fill="#00AA13" stroke="#00AA13" stroke-width="2.66665" stroke-linecap="round" stroke-linejoin="round"/>
<defs>
<filter id="filter0_f_3308_322" x="0.14822" y="0.650173" width="36.7026" height="36.7036" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
<feFlood flood-opacity="0" result="BackgroundImageFix"/>
<feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape"/>
<feGaussianBlur stdDeviation="4.5714" result="effect1_foregroundBlur_3308_322"/>
</filter>
</defs>`
		},
		idle_status_icon : {
			box: 38,
			svg:  `<g filter="url(#filter0_f_3308_322)">
<ellipse cx="18.4995" cy="19.0021" rx="7.875" ry="7.87511" stroke="#2E323C" stroke-width="2.66665" stroke-linecap="round" stroke-linejoin="round"/>
</g>
<ellipse cx="18.5" cy="19.002" rx="5.25" ry="5.25007" fill="#2E323C" stroke="#2E323C" stroke-width="2.66665" stroke-linecap="round" stroke-linejoin="round"/>
<defs>
<filter id="filter0_f_3308_322" x="0.14822" y="0.650173" width="36.7026" height="36.7036" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
<feFlood flood-opacity="0" result="BackgroundImageFix"/>
<feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape"/>
<feGaussianBlur stdDeviation="4.5714" result="effect1_foregroundBlur_3308_322"/>
</filter>
</defs>`
		},
		add_square: {
			box: 33,
			svg: `<rect x="31.15" y="31.15" width="29.3" height="29.3" rx="7.79286" transform="rotate(-180 31.15 31.15)" stroke="#2D8AFF" stroke-width="2.7"/><path d="M24.0439 16.0429L9.41538 16.0429M16.7297 8.72861L16.7297 23.3572" stroke="#2D8AFF" stroke-width="2.7" stroke-linecap="round" stroke-linejoin="round"/>`
		}
	} as const;
	let displayIcon = icons[name];
</script>

<svg
	fill="none"
	class={$$props.class}
	{focusable}
	{width}
	{height}
	viewBox="0 0 {displayIcon.box} {displayIcon.box}">{@html displayIcon.svg}</svg
>





