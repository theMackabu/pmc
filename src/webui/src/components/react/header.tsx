export default (props: { name: string; description: string; children }) => (
	<div class="sm:flex sm:items-center p-4 sm:p-6 lg:p-8">
		<div class="sm:flex-auto">
			<h1 class="text-base font-semibold leading-6 text-white">{props.name}</h1>
			<p class="mt-2 text-sm text-zinc-300">{props.description}</p>
		</div>
		<div class="mt-4 sm:ml-16 sm:mt-0 sm:flex-none">{props.children}</div>
	</div>
);
