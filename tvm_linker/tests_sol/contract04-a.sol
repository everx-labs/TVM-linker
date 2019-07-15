pragma solidity ^0.5.0;

contract MyContract {

	uint m_counter;

	function method(MyContract anotherContract, uint amount) public {
		// call remote contract
		anotherContract.remoteMethod(amount);
		m_counter = m_counter + 1;
		return;
	}
	
	// Dummy function for a remote contract. Later to be represented by a distinct interface
	
	function remoteMethod(uint value) pure public {
		require(value > 0);
		return; 
	}
	
}
