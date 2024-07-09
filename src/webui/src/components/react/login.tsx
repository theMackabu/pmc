import { $settings } from '@/store';
import styled from '@emotion/styled';
import { useState, useEffect } from 'react';

const LoginBackground = styled.div`
	&:before {
		content: '';
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		height: 100vh;
		top: 0;

		opacity: ${(props) => (props.show ? 1 : 0)};
		background-image: url('${(props) => props.basePath}/assets/login.svg');

		background-position: top center;
		background-size: auto;
		background-repeat: no-repeat;
		transition: opacity 3s ease-in-out;

		@media only screen and (min-width: 768px) {
			background-position: center;
			background-size: cover;
		}
	}
`;

const Login = (props: { base: string }) => {
	const [token, setToken] = useState('');
	const [startAnim, setStartAnim] = useState(false);
	const [loginFailed, setLoginFailed] = useState(false);

	const handleChange = (event: any) => setToken(event.target.value);

	const handleSubmit = (event: any) => {
		event.preventDefault();
		$settings.setKey('token', token);

		fetch(props.base + '/daemon/metrics', {
			headers: { token }
		}).then((response) => {
			if (response.status === 200) {
				window.location.href = props.base;
			} else {
				setLoginFailed(true);
				setTimeout(() => {
					setLoginFailed(false);
				}, 3000);
			}
		});
	};

	useEffect(() => {
		setTimeout(() => {
			setStartAnim(true);
		}, 100);
	}, []);

	return (
		<LoginBackground basePath={props.base} show={startAnim}>
			{loginFailed && (
				<div className="-mb-[92px] sm:mx-auto sm:w-full sm:max-w-[480px] sm:rounded-lg bg-red-600 sm:border border-red-400/50 p-4 sm:mt-4 sm:-mb-[110px]">
					<h3 className="text-sm font-medium text-white">Failed to login. Is the token correct?</h3>
				</div>
			)}
			<div className="h-screen flex min-h-full flex-1 flex-col justify-center px-6 py-12 lg:px-8 -mt-12">
				<div className="mt-10 sm:mx-auto sm:w-full sm:max-w-sm bg-zinc-900/70 backdrop-blur-md px-5 py-6 rounded-lg border border-zinc-800 transition-all shadow-xl">
					<div className="flex min-h-full flex-1 flex-col justify-center">
						<div className="mb-5">
							<img className="h-10 w-auto" src={`${props.base}/assets/favicon.svg`} alt="PMC" />
							<h2 className="mt-6 text-2xl font-bold leading-9 tracking-tight text-white">Welcome back</h2>
							<p className="mt-1.5 text-sm leading-6 text-zinc-300">Sign in to your account</p>
						</div>
						<form className="space-y-6" onSubmit={handleSubmit}>
							<div>
								<div>
									<label htmlFor="password" className="block text-sm font-medium leading-6 text-zinc-300 mb-1">
										Password
									</label>
									<input
										required
										id="password"
										name="password"
										type="password"
										value={token}
										onChange={handleChange}
										placeholder="••••••••••••••••"
										autoComplete="current-password"
										className="block w-full rounded-md border-0 bg-white/5 py-2 text-white shadow-sm ring-1 ring-inset ring-white/10 focus:ring-2 focus:ring-inset focus:ring-sky-500 sm:text-sm sm:leading-6 placeholder:text-zinc-600"
									/>
								</div>
							</div>

							<div>
								<button
									type="submit"
									className="-mb-1 transition flex w-full justify-center rounded-md px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 border focus:outline-none focus:ring-0 focus:ring-offset-0 focus:z-10 shrink-0 saturate-[110%] border-zinc-700 hover:border-zinc-600 bg-zinc-800/60 text-zinc-50 hover:bg-zinc-700/60">
									Continue
								</button>
							</div>
						</form>
					</div>
				</div>
			</div>
		</LoginBackground>
	);
};

export default Login;
