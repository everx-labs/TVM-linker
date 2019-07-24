pragma solidity ^0.5.0;

// the interface of a remove contract
contract AnotherContract {
    function remoteMethod(uint64 value) pure public;
}

// the contract calling the remote method
contract MyContract {

    // persistent variable storing the number of function 'method' was called
    uint m_counter;

    function method(AnotherContract anotherContract) public {
        // call function of remote contract
        anotherContract.remoteMethod(257);
        m_counter = m_counter + 1; // incrementing the counter
        return;
    }

}
