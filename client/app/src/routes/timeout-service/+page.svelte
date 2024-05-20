<script lang="ts">
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';
	import hourglass from '$lib/images/hourglass.png';
	const goBack = () => {
		history.back();
	};
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col justify-center items-center gap-2 mt-8">
			<div class="mb-2.5">
				<img alt="check internet" src={hourglass} />
			</div>
			<div class="flex justify-center text-lg">
				<span> Request timed out, please try again </span>
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-end px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#2A2A2C] p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="refresh_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
