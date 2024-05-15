<script>
	import { Toaster } from 'svelte-french-toast';
	import '../app.pcss';
</script>

<div class="">
	<Toaster
		toastOptions={{
			style: 'text-transform: capitalize; font-size: 16px; font-weight: 500; color: #B7BBC8; background-color: rgba(255, 255, 255, 0.2);',
            position: 'bottom-center'
		}}
	/>
	<slot></slot>
</div>
