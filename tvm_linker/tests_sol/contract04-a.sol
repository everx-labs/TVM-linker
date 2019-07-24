pragma solidity ^0.5.0;

contract AnotherContract {
        function remoteMethod(uint value) pure public;
}

contract MyContract {

	uint m_counter;

        function method(AnotherContract anotherContract, uint amount) public {
		// call remote contract
		anotherContract.remoteMethod(amount);
		m_counter = m_counter + 1;
		return;
        }

}
