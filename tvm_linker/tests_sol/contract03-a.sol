pragma solidity ^0.5.0;

contract AnotherContract {
        function remoteMethod() pure public;
}

contract MyContract {

	uint m_counter;

        function method(AnotherContract anotherContract) public {
		// call remote contract
		anotherContract.remoteMethod();
		m_counter = m_counter + 1;
		return;
	}
	
}
