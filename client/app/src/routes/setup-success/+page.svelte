<script lang="ts">
	import Success from '$lib/images/gifs/Success.gif';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';
	import { goto } from '$app/navigation';
	import { goBack } from '$lib/services';

	const goNext = () => {
		goto('/machine-info');
	};
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" flex flex-grow flex-col items-center justify-center gap-2">
			<div class="m-6">
				<img alt="check internet" src={Success} />
			</div>
			<div class="flex justify-center text-2xl">
				<!-- Machine is now connected to your Mecha account -->
				Setup complete!
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div
			class="border-silver-gray flex h-full w-full flex-row items-center justify-between border-t-2 px-4 py-3"
		>
			<button
				class="flex h-[60px] w-[60px] items-center justify-center rounded-lg p-1 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="left_arrow" width="60" height="60" />
			</button>
			<button
				class="flex h-[60px] w-[60px] items-center justify-center rounded-lg p-2 text-[#FAFBFC]"
				on:click={goNext}
			>
				<Icons name="right_arrow" width="60" height="60" />
			</button>
		</div>
	</footer>
</Layout>
