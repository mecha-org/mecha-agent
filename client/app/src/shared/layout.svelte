<main class={`m-0 p-0  ${$$slots.footer ? 'pb-20' : 'pb-5'} pl-4 pr-4 pt-4 min-h-screen `}><slot /></main>

{#if $$slots.footer}
	<footer
		style="height: 80px; position: fixed; bottom: 0; width: 100%; text-align: center; z-index: 10;"
	>
		<slot name="footer" />
	</footer>
{/if}

<style>
	header {
		margin: 0;
		padding: 0;
	}

	footer {
		text-align: center;
	}
</style>