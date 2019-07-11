pragma solidity ^0.5.0;

contract AnotherContract {
	function remoteMethod(uint64 value) pure public;
}

contract MyContract {

	uint m_counter;

	function method(AnotherContract anotherContract) public {
		// call remote contract
		anotherContract.remoteMethod(257);
		m_counter = m_counter + 1;
		return;
	}
	
}
